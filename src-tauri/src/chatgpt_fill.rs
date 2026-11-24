//! A narrow, user-initiated bridge to the composer of the official ChatGPT webpage.
//! The local app's main view is the only view granted invoke permissions.
//! The script never retrieves messages/cookies, never calls ChatGPT APIs and never sends.
use std::{sync::mpsc, time::Duration};
use tauri::Manager;

const LABEL_PREFIX: &str = "chatgpt-embedded-";
const MAX_PROMPT_CHARS: usize = 20_900;
const RETRY_COUNT: usize = 16;

async fn short_delay() {
    let _ = tauri::async_runtime::spawn_blocking(|| {
        std::thread::sleep(Duration::from_millis(550));
    }).await;
}

#[tauri::command]
pub async fn chatgpt_fill_prompt(app: tauri::AppHandle, label: String, prompt: String)
    -> Result<String, String>
{
    let suffix = label.strip_prefix(LABEL_PREFIX).ok_or("非法 ChatGPT 窗口标识")?;
    if suffix.len() != 16 || !suffix.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("非法 ChatGPT 窗口标识".into());
    }
    if prompt.trim().is_empty() || prompt.chars().count() > MAX_PROMPT_CHARS || prompt.contains('\0') {
        return Err("科研提示词为空或超过大小限制".into());
    }
    let literal = serde_json::to_string(&prompt).map_err(|_| "提示词编码失败".to_owned())?;
    let script = include_str!("chatgpt_prompt_fill.js").replace("__PROMPT_JSON__", &literal);
    for _ in 0..RETRY_COUNT {
        let view = match app.get_webview(&label) {
            Some(view) => view,
            None => return Ok("closed".into()),
        };
        let url = match view.url() {
            Ok(url) => url,
            Err(_) => { short_delay().await; continue; }
        };
        if url.scheme() == "about" || url.as_str().is_empty() {
            short_delay().await;
            continue;
        }
        if url.scheme() != "https" || url.host_str() != Some("chatgpt.com") {
            if url.host_str().is_some_and(|host| host == "auth.openai.com" || host == "auth0.openai.com") {
                return Ok("login_required".into());
            }
            return Ok("wrong_origin".into());
        }
        // eval_with_callback gives a status code only: no page content is returned.
        let (sender, receiver) = mpsc::sync_channel::<String>(1);
        view.eval_with_callback(script.clone(), move |result| { let _ = sender.send(result); })
            .map_err(|_| "无法在 ChatGPT WebView2 中执行输入动作".to_owned())?;
        let callback = tauri::async_runtime::spawn_blocking(move || {
            receiver.recv_timeout(Duration::from_secs(3))
        }).await.map_err(|_| "输入结果检查失败".to_owned())?;
        let raw = match callback {
            Ok(value) => value,
            Err(_) => { short_delay().await; continue; },
        };
        let code: String = serde_json::from_str(&raw).unwrap_or(raw);
        match code.as_str() {
            "filled" | "already_filled" | "occupied" | "unsupported" | "wrong_origin" => return Ok(code),
            "not_ready" => short_delay().await,
            _ => return Ok("unsupported".into()),
        }
    }
    Ok("not_ready".into())
}
