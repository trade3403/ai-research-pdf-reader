# D1.13b: hosted Windows deployment test. No installation or user machine.
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [Console]::OutputEncoding

# Start from the release binaries and verified model artifacts only.
$deploy = Join-Path $env:RUNNER_TEMP 'RP 部署 测试'
New-Item -ItemType Directory -Path $deploy | Out-Null
$auto = Join-Path $deploy 'rptranslate.exe'
$portable = Join-Path $deploy 'rptranslate-portable.exe'
Copy-Item -LiteralPath 'target/release/rptranslate.exe' -Destination $auto
Copy-Item -LiteralPath 'target/portable/release/rptranslate.exe' -Destination $portable
$fp32 = Join-Path $deploy '模型 FP32'
$int8 = Join-Path $deploy '模型 INT8'
Move-Item -LiteralPath (Join-Path $env:RUNNER_TEMP 'rpmodel-v1') -Destination $fp32
Move-Item -LiteralPath (Join-Path $env:RUNNER_TEMP 'rpmodel-mixed') -Destination $int8

# Inspect static PE imports with the native MSVC toolchain. Explicit system/CRT
# allowlist; do not infer a clean-machine runtime guarantee from this runner.
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vs) { throw 'MSVC installation not found' }
$dumpbin = Get-ChildItem -LiteralPath (Join-Path $vs 'VC/Tools/MSVC') -Directory |
    Sort-Object Name -Descending |
    ForEach-Object { Join-Path $_.FullName 'bin/Hostx64/x64/dumpbin.exe' } |
    Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $dumpbin) { throw 'dumpbin not found' }
$system = @('kernel32.dll','kernelbase.dll','ntdll.dll','advapi32.dll','bcrypt.dll',
    'bcryptprimitives.dll','crypt32.dll','userenv.dll','ws2_32.dll','secur32.dll',
    'ole32.dll','oleaut32.dll','shell32.dll','shlwapi.dll','user32.dll','gdi32.dll',
    'combase.dll','psapi.dll','version.dll','dbghelp.dll','iphlpapi.dll','msvcrt.dll',
    'ucrtbase.dll','vcruntime140.dll','vcruntime140_1.dll','msvcp140.dll',
    'normaliz.dll','rpcrt4.dll','api-ms-win-core-synch-l1-2-0.dll')
foreach ($exe in @($auto, $portable)) {
    $dump = & $dumpbin /DEPENDENTS $exe
    $imports = @($dump | ForEach-Object {
        if ($_ -match '^\s+([A-Za-z0-9_.-]+\.dll)\s*$') { $Matches[1].ToLowerInvariant() }
    } | Sort-Object -Unique)
    if ($imports.Count -eq 0) { throw 'No PE imports parsed' }
    foreach ($dll in $imports) {
        if ($dll -notin $system -and $dll -notmatch '^api-ms-win-(core|crt)-[a-z0-9-]+\.dll$') {
            throw "Unexpected static dependency: $dll"
        }
        if ($dll -notmatch '^api-ms-win-' -and -not (Test-Path -LiteralPath (Join-Path "$env:SystemRoot/System32" $dll))) {
            throw "Missing system dependency: $dll"
        }
    }
    Write-Output "WINDOWS_DEPENDENCIES PASS exe=$([IO.Path]::GetFileName($exe)) imports=$($imports -join ',')"
}

# Check the actual executable under a program-scoped outbound firewall block.
# This tests no external network dependency, not kernel-level isolation.
$profiles = @(Get-NetFirewallProfile -PolicyStore ActiveStore)
if ($profiles.Count -ne 3 -or @($profiles | Where-Object { $_.Enabled -ne 'True' }).Count -ne 0) {
    throw 'Windows Firewall must be enabled for all profiles'
}
$rules = @()
Push-Location -LiteralPath $deploy
try {
    foreach ($exe in @($auto, $portable)) {
        $name = "RPTranslate-CI-$([Guid]::NewGuid())"
        New-NetFirewallRule -Name $name -DisplayName $name -Direction Outbound -Action Block -Program $exe -Profile Any -Enabled True | Out-Null
        $rules += $name
        $active = Get-NetFirewallRule -PolicyStore ActiveStore -Name $name
        $app = $active | Get-NetFirewallApplicationFilter
        if ($active.Enabled -ne 'True' -or $active.Action -ne 'Block' -or $active.Direction -ne 'Outbound' -or $app.Program -ne $exe) {
            throw 'Outbound rule not active for exact deployed executable'
        }
    }
    foreach ($spec in @(@{Name='fp32'; Root=$fp32; Bytes=312033188; Command='translate-paragraph'},
                        @{Name='int8'; Root=$int8; Bytes=84903620; Command='translate-paragraph-int8'})) {
        $name = $spec.Name
        $root = $spec.Root
        $cmd = $spec.Command
        $a = (& $auto model-check $name $root) -join ' '
        $b = (& $portable model-check $name $root) -join ' '
        $prefix = "MODEL_CHECK PASS precision=$name payload_bytes=$($spec.Bytes) tokens="
        if (-not $a.StartsWith($prefix) -or -not $b.StartsWith($prefix) -or -not $b.EndsWith('avx2=false')) {
            throw "Model verification mismatch: $a / $b"
        }
        Write-Output $a
        Write-Output $b
        $source = 'We measured maximum strain.'
        $a = (& $auto $cmd $root geotechnical $source 1 128) -join "\n"
        $b = (& $portable $cmd $root geotechnical $source 1 128) -join "\n"
        if ($a -cne $b -or -not $a.Contains('最大应变')) { throw "Translation mismatch: $name" }
        Write-Output "WINDOWS_DEPLOY PASS precision=$name auto_portable_equal=true unicode_path=true outbound_block=true"
    }
    # Fail closed on actual FP32 package corruption; restore and reverify.
    $weights = Join-Path $fp32 'weights.safetensors'
    $stream = [IO.File]::Open($weights, 'Open', 'ReadWrite', 'None')
    try {
        $original = $stream.ReadByte()
        if ($original -lt 0) { throw 'Empty FP32 weight file' }
        $stream.Position = 0
        $stream.WriteByte([byte]($original -bxor 1))
    } finally { $stream.Dispose() }
    try {
        $PSNativeCommandUseErrorActionPreference = $false
        & $auto model-check fp32 $fp32
        if ($LASTEXITCODE -eq 0) { throw 'Damaged FP32 model accepted' }
    } finally {
        $PSNativeCommandUseErrorActionPreference = $true
        $stream = [IO.File]::Open($weights, 'Open', 'ReadWrite', 'None')
        try { $stream.WriteByte([byte]$original) } finally { $stream.Dispose() }
    }
    & $auto model-check fp32 $fp32
    Write-Output 'WINDOWS_FP32_NEGATIVE PASS restored=true'
} finally {
    Pop-Location
    foreach ($name in $rules) { Remove-NetFirewallRule -Name $name }
}
