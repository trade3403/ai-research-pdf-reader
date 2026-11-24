use std::{fs, path::{Path,PathBuf}, io::{Read, BufReader}};
use sha2::{Digest, Sha256};
use serde::Serialize;
use walkdir::WalkDir;
use tauri::Manager;
mod chatgpt_fill;

const PDF_MAX_BYTES: u64 = 100 * 1024 * 1024;
const MAX_DISCOVERED_FILES: usize = 20_000;

/// Restores a local PDF previously opened by the user. V0.2 intentionally does not
/// support URLs or any non-PDF file format. No PDF data is sent to the network.
#[tauri::command]
fn read_recent_pdf(path: String) -> Result<Vec<u8>, String> {
    let pdf_path = Path::new(&path);
    if !pdf_path.is_absolute() || !pdf_path.is_file() ||
        !pdf_path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("pdf")) {
        return Err("保存的 PDF 路径无效或文件已不存在".to_owned());
    }
    let size = fs::metadata(pdf_path).map_err(|e| e.to_string())?.len();
    if size > PDF_MAX_BYTES || size < 5 {
        return Err("PDF 文件大小不符合 V0.2 限制".to_owned());
    }
    let bytes = fs::read(pdf_path).map_err(|e| e.to_string())?;
    if !bytes.starts_with(b"%PDF-") {
        return Err("不是有效的 PDF 文件".to_owned());
    }
    Ok(bytes)
}

/// Markdown is exported only to a path explicitly chosen by the Save dialog.
#[tauri::command]
fn write_markdown(path: String, content: String) -> Result<(), String> {
    let output = Path::new(&path);
    if !output.is_absolute() || !output.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("md")) {
        return Err("只支持保存为绝对路径的 .md 文件".to_owned());
    }
    if content.len() > 10 * 1024 * 1024 {
        return Err("笔记导出内容过大".to_owned());
    }
    fs::write(output, content).map_err(|e| format!("保存失败：{e}"))
}


#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InspectedPdf { path: String, filename: String, sha256: String, size_bytes: u64 }

#[tauri::command]
fn inspect_pdf_file(path: String) -> Result<InspectedPdf, String> {
    let canonical = Path::new(&path).canonicalize().map_err(|e| format!("文件无法访问：{e}"))?;
    if !canonical.is_file() || !canonical.extension().and_then(|s|s.to_str()).is_some_and(|s|s.eq_ignore_ascii_case("pdf")) {
        return Err("只能导入 PDF 文件".to_owned());
    }
    let len = fs::metadata(&canonical).map_err(|e|e.to_string())?.len();
    if len < 5 || len > PDF_MAX_BYTES { return Err("文件大小不符合 100 MB 限制".to_owned()); }
    let mut file=BufReader::new(fs::File::open(&canonical).map_err(|e|e.to_string())?);
    let mut header=[0u8;5];file.read_exact(&mut header).map_err(|e|e.to_string())?;
    if &header != b"%PDF-" {return Err("PDF 文件头无效".to_owned());}
    let mut hasher=Sha256::new();hasher.update(header);
    let mut buffer=[0u8;65536];
    loop {let n=file.read(&mut buffer).map_err(|e|e.to_string())?;if n==0{break;}hasher.update(&buffer[..n]);}
    let filename=canonical.file_name().and_then(|x|x.to_str()).unwrap_or("unknown.pdf").to_owned();
    Ok(InspectedPdf{path:canonical.to_string_lossy().to_string(),filename,sha256:format!("{:x}",hasher.finalize()),size_bytes:len})
}

#[tauri::command]
fn list_pdf_paths(root: String) -> Result<Vec<String>, String> {
    let root=PathBuf::from(root).canonicalize().map_err(|e|e.to_string())?;
    if !root.is_dir() {return Err("所选路径不是文件夹".to_owned());}
    let mut found=Vec::new();
    for entry in WalkDir::new(root).max_depth(12).follow_links(false).into_iter() {
        let e=match entry {Ok(e)=>e,Err(_)=>continue};
        if e.file_type().is_file() && e.path().extension().and_then(|x|x.to_str()).is_some_and(|x|x.eq_ignore_ascii_case("pdf")) {
            found.push(e.path().to_string_lossy().to_string());
            if found.len() >= MAX_DISCOVERED_FILES {return Err("单次最多扫描 20,000 个 PDF，请分批导入".to_owned());}
        }
    }
    found.sort(); Ok(found)
}

/// Only writes to a filename chosen in a native Save dialog. Never modifies source PDFs.
#[tauri::command]
fn write_library_export(path:String,content:String)->Result<(),String>{
    let output=Path::new(&path);
    let allowed=output.extension().and_then(|s|s.to_str()).is_some_and(|x|["md","bib","ris","json","txt"].iter().any(|y|x.eq_ignore_ascii_case(y)));
    if !output.is_absolute()||!allowed {return Err("导出路径或后缀无效".to_string());}
    if content.len()>20*1024*1024 {return Err("导出内容超过 20 MB".to_string());}
    fs::write(output,content).map_err(|e|e.to_string())
}

#[tauri::command]
fn read_citation_import(path:String)->Result<String,String>{
    let file=Path::new(&path);
    if !file.is_absolute() || !file.is_file() || !file.extension().and_then(|s|s.to_str()).is_some_and(|x|["bib","ris"].iter().any(|y|x.eq_ignore_ascii_case(y))){return Err("只能导入 .bib 或 .ris".to_string());}
    if fs::metadata(file).map_err(|e|e.to_string())?.len()>5*1024*1024{return Err("引用文件不得超过 5 MB".to_string());}
    fs::read_to_string(file).map_err(|e|e.to_string())
}



// Azure Translator F0 is a user-configured optional service; the remote ChatGPT
// child webview has no Tauri capability, so it cannot call these commands.
// The credential lives in Windows Credential Manager, never in SQLite or frontend.
const TRANSLATOR_SERVICE: &str = "ai-research-pdf-reader.azure-translator";
const TRANSLATOR_ACCOUNT: &str = "default";
fn translation_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(TRANSLATOR_SERVICE, TRANSLATOR_ACCOUNT)
        .map_err(|_| "Windows 凭据管理器不可用".to_owned())
}

#[tauri::command]
fn translator_save_key(key: String) -> Result<(), String> {
    if key.len() < 8 || key.len() > 256 || (key.contains('\r') || key.contains('\n')) {
        return Err("Azure Key 格式不正确".to_owned());
    }
    translation_entry()?.set_password(&key)
        .map_err(|_| "Azure Key 保存到 Windows 凭据管理器失败".to_owned())
}

#[tauri::command]
fn translator_has_key() -> Result<bool, String> {
    match translation_entry()?.get_password() {
        Ok(value) => Ok(!value.is_empty()),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(_) => Err("无法读取 Windows 凭据管理器".to_owned()),
    }
}

#[tauri::command]
fn translator_remove_key() -> Result<(), String> {
    match translation_entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err("无法删除 Windows 凭据管理器中的 Azure Key".to_owned()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TranslatorAnswer { translations: Vec<String> }

#[derive(serde::Deserialize)]
struct AzureTranslated { text: String }
#[derive(serde::Deserialize)]
struct AzureRecord { translations: Vec<AzureTranslated> }

#[tauri::command]
async fn translator_translate(texts: Vec<String>, region: String, from: String, to: String)
    -> Result<TranslatorAnswer, String> {
    if texts.is_empty() || texts.len() > 100 ||
       texts.iter().any(|t| t.is_empty() || t.chars().count() > 10_000) ||
       texts.iter().map(|t| t.chars().count()).sum::<usize>() > 10_000 {
        return Err("翻译请求不能超过 100 段、共 1 万字符".into());
    }
    if !["auto", "en", "zh-Hans"].contains(&from.as_str()) ||
        !["en", "zh-Hans"].contains(&to.as_str()) ||
        region.is_empty() || region.len() > 45 || !region.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-') {
        return Err("翻译参数无效".into());
    }
    let key = translation_entry()?.get_password().map_err(|_| "请先在翻译设置中保存 Azure Translator Key".to_owned())?;
    let mut url = format!("https://api.cognitive.microsofttranslator.com/translate?api-version=3.0&to={to}");
    if from != "auto" { url.push_str(&format!("&from={from}")); }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(35)).redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_| "无法创建 Azure 网络客户端".to_owned())?;
    let mut req = client.post(url)
        .header("Ocp-Apim-Subscription-Key", key)
        .header(reqwest::header::CONTENT_TYPE, "application/json; charset=utf-8");
    if region != "global" {
        req = req.header("Ocp-Apim-Subscription-Region", region);
    }
    let body: Vec<serde_json::Value> = texts.iter().map(|t| serde_json::json!({"Text":t})).collect();
    let response = req.json(&body).send().await
        .map_err(|_| "无法连接 Azure Translator，请检查网络与区域设置".to_owned())?;
    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => "Azure Key、区域或资源权限不正确".to_owned(),
            429 => "Azure 翻译额度或速率已达到限制，请查看 Azure 门户".to_owned(),
            400 => "Azure 拒绝翻译请求，请检查语言和文本格式".to_owned(),
            408 | 503 => "Azure 翻译服务暂时不可用，请稍后重试".to_owned(),
            _ => format!("Azure Translator 请求失败（HTTP {}）", status.as_u16()),
        });
    }
    let rows = response.json::<Vec<AzureRecord>>().await
        .map_err(|_| "Azure 译文响应解析失败".to_owned())?;
    if rows.len() != texts.len() { return Err("Azure 返回片段数不匹配".to_owned()); }
    let mut translations = Vec::with_capacity(rows.len());
    for row in rows {
        let translation = row.translations.into_iter().next()
            .ok_or("Azure 未返回指定语言的译文")?;
        translations.push(translation.text);
    }
    Ok(TranslatorAnswer{translations})
}

// A SQLite VACUUM INTO backup is made by the trusted local WebView via the
// already-established plugin-sql connection. The native command checks the
// user-selected target before SQLite creates the consistent snapshot.
#[tauri::command]
fn backup_prepare_target(path: String) -> Result<(), String> {
    let target=Path::new(&path);
    if !target.is_absolute() || !target.extension().and_then(|x|x.to_str())
        .is_some_and(|x|x.eq_ignore_ascii_case("sqlite")) {
        return Err("请选择绝对路径的 .sqlite 备份文件".into());
    }
    if target.exists() { return Err("备份文件已存在，请选择新文件名以防覆盖旧备份".into()); }
    if !target.parent().is_some_and(|dir|dir.is_dir()) { return Err("备份目录不存在".into()); }
    Ok(())
}

const DB_FILENAME: &str = "research-reader-v02.db";
const RESTORE_STAGE: &str = "research-reader-pending-restore.sqlite";
const MAX_DB_BACKUP: u64 = 512*1024*1024;

fn looks_like_sqlite(path: &Path) -> Result<(), String> {
    let size=fs::metadata(path).map_err(|e|format!("备份无法访问：{e}"))?.len();
    if size < 100 || size > MAX_DB_BACKUP { return Err("备份文件必须是小于 512 MB 的有效 SQLite 数据库".into()); }
    let mut header=[0u8;16];
    fs::File::open(path).and_then(|mut f|f.read_exact(&mut header))
        .map_err(|_|"无法读取备份文件头".to_owned())?;
    if &header != b"SQLite format 3\0" { return Err("所选文件不是 SQLite 数据库".into()); }
    Ok(())
}

/// Stage a user-selected backup only. Never replace a live SQLite file.
/// Restore happens on the NEXT launch, before the frontend opens SQLite.
#[tauri::command]
fn stage_database_restore(app: tauri::AppHandle, path:String) -> Result<(), String> {
    let source=Path::new(&path);
    if !source.is_absolute() || !source.is_file() || !source.extension().and_then(|x|x.to_str())
        .is_some_and(|x|x.eq_ignore_ascii_case("sqlite")) {
        return Err("请选择 .sqlite 数据库备份".into());
    }
    looks_like_sqlite(source)?;
    let dir=app.path().app_config_dir().map_err(|e|e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e|e.to_string())?;
    let staged=dir.join(RESTORE_STAGE);
    if source == staged { return Err("不能使用待恢复文件本身作为来源".into()); }
    let tmp=dir.join("research-reader-pending-restore.tmp");
    fs::copy(source,&tmp).map_err(|e|format!("备份复制失败：{e}"))?;
    looks_like_sqlite(&tmp)?;
    if staged.exists() {fs::remove_file(&staged).map_err(|e|e.to_string())?;}
    fs::rename(&tmp,&staged).map_err(|e|e.to_string())?;
    Ok(())
}

/// Runs before the SQL plugin frontend obtains a database connection.
/// The current database is copied to a rollback file before replacement.
fn restore_staged_database(app:&mut tauri::App) -> Result<(),String> {
    let dir=app.path().app_config_dir().map_err(|e|e.to_string())?;
    let staged=dir.join(RESTORE_STAGE);
    if !staged.exists() {return Ok(());}
    looks_like_sqlite(&staged)?;
    let live=dir.join(DB_FILENAME);
    let rollback=dir.join("research-reader-before-restore.sqlite");
    if live.exists() {
        fs::copy(&live,&rollback).map_err(|e|format!("无法创建恢复前备份：{e}"))?;
    }
    let replacement=dir.join("research-reader-restored.tmp");
    fs::copy(&staged,&replacement).map_err(|e|e.to_string())?;
    looks_like_sqlite(&replacement)?;
    // The process is at app startup; no frontend SQLite connections exist yet.
    // Remove sidecars from the old database after the rollback copy exists.
    // The final replacement remains staged until every prerequisite succeeds.
    if live.exists() {fs::remove_file(&live).map_err(|e|e.to_string())?;}
    if let Err(e)=fs::rename(&replacement,&live) {
        if rollback.exists() {let _=fs::copy(&rollback,&live);}
        return Err(format!("无法完成恢复：{e}"));
    }
    for suffix in ["-wal","-shm"] {
        let sidecar=dir.join(format!("{DB_FILENAME}{suffix}"));
        if sidecar.exists() {let _=fs::remove_file(sidecar);}
    }
    let _=fs::remove_file(staged);
    Ok(())
}

/// Store an explicitly exported Word file. Native save dialog controls the destination.
/// A sibling backup is left when replacing an existing DOCX, so the original can
/// be recovered after a failed or interrupted overwrite.
#[tauri::command]
fn write_word_docx(path: String, bytes: Vec<u8>) -> Result<(), String> {
    let target = Path::new(&path);
    if !target.is_absolute() || !target.extension().and_then(|e|e.to_str()).is_some_and(|e|e.eq_ignore_ascii_case("docx")) {
        return Err("只能将 DOCX 保存到用户选择的绝对路径".into());
    }
    if bytes.len() < 4 || bytes.len() > 32 * 1024 * 1024 || bytes[0] != b'P' || bytes[1] != b'K' {
        return Err("DOCX 文件头无效或超过 32 MB 大小限制".into());
    }
    let parent = target.parent().ok_or("缺少父目录")?;
    if !parent.is_dir() { return Err("目标文件夹不存在".into()); }
    let pending = target.with_extension("docx.tmp");
    let backup = target.with_extension("docx.before-save.bak");
    fs::write(&pending, &bytes).map_err(|e|format!("写入临时 DOCX 失败：{e}"))?;
    if target.exists() {
        if backup.exists() { fs::remove_file(&backup).map_err(|e|e.to_string())?; }
        fs::copy(target, &backup).map_err(|e|format!("创建 DOCX 安全副本失败：{e}"))?;
        fs::remove_file(target).map_err(|e|format!("替换 DOCX 失败：{e}"))?;
    }
    if let Err(err) = fs::rename(&pending, target) {
        if backup.exists() { let _ = fs::copy(&backup, target); }
        return Err(format!("完成 DOCX 保存失败：{err}"));
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_sql::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Err(error)=restore_staged_database(app) {
                eprintln!("Research Reader database restore was not completed: {error}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![read_recent_pdf, write_markdown, inspect_pdf_file, list_pdf_paths, write_library_export, read_citation_import, translator_save_key, translator_has_key, translator_remove_key, translator_translate, backup_prepare_target, stage_database_restore, write_word_docx, chatgpt_fill::chatgpt_fill_prompt])
        .run(tauri::generate_context!())
        .expect("failed to launch AI Research PDF Reader");
}
