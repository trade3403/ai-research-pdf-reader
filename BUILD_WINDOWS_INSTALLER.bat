@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
echo ============================================
echo AI Research PDF Reader - Windows installer
echo ============================================
echo.
where powershell >nul 2>nul
if errorlevel 1 (
    echo ERROR: Windows PowerShell is required.
    pause
    exit /b 1
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-windows.ps1"
set EXITCODE=%ERRORLEVEL%
echo.
if %EXITCODE% EQU 0 (
    echo SUCCESS - Find your setup EXE in INSTALLER_OUTPUT
    explorer "%~dp0INSTALLER_OUTPUT"
) else (
    echo Build FAILED. Read build-logs for errors.
    echo See WINDOWS_INSTALLER_GUIDE.md
)
pause
exit /b %EXITCODE%
