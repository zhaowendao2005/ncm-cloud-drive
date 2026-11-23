use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_dialog::DialogExt;

use crate::api::{auth, Endpoint};
use crate::engine::upload;
use crate::state::{save_credentials, AppState, SharedState};
use crate::store::history;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateInfo {
    has_credentials: bool,
    logged_in: bool,
    nickname: Option<String>,
    avatar: Option<String>,
    device_id: String,
}

fn endpoint_parts(st: &AppState) -> (String, rsa::RsaPrivateKey, String) {
    let app_id = st.app_id.lock().unwrap().clone();
    let key = st.key.lock().unwrap().clone();
    let device_json = st.device.to_json();
    (app_id, key, device_json)
}

#[tauri::command]
pub async fn get_state(state: State<'_, SharedState>) -> Result<StateInfo, String> {
    let st = state.inner().clone();
    let logged_in = st.token_valid();
    let profile = st.profile.lock().unwrap().clone();
    Ok(StateInfo {
        has_credentials: st.has_credentials(),
        logged_in,
        nickname: profile
            .as_ref()
            .and_then(|p| p.get("nickname"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        avatar: profile
            .as_ref()
            .and_then(|p| p.get("avatarUrl"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        device_id: st.device.device_id.clone(),
    })
}

#[tauri::command]
pub async fn save_credentials_cmd(
    app: AppHandle,
    state: State<'_, SharedState>,
    app_id: String,
    private_key: String,
) -> Result<(), String> {
    let st = state.inner().clone();
    save_credentials(&st.app_dir, &app_id, &private_key)?;
    *st.app_id.lock().unwrap() = app_id.trim().to_string();
    *st.key.lock().unwrap() = crate::state::parse_key(&private_key)?;
    upload::emit_log(&app, "success", "应用凭据已保存到本机", Some("config".into()));
    Ok(())
}

#[tauri::command]
pub async fn get_profile(state: State<'_, SharedState>) -> Result<Option<Value>, String> {
    let st = state.inner().clone();
    let token = match st.access_token() {
        Some(t) => t,
        None => return Ok(None),
    };
    let (app_id, key, device_json) = endpoint_parts(&st);
    let ep = Endpoint {
        http: &st.http,
        key: &key,
        app_id: &app_id,
        device: &device_json,
        device_id: &st.device.device_id,
    };
    let profile = auth::profile(&ep, &token).await?;
    if profile.is_null() {
        return Ok(None);
    }
    *st.profile.lock().unwrap() = Some(profile.clone());
    Ok(Some(profile))
}

#[tauri::command]
pub async fn login_start(app: AppHandle, state: State<'_, SharedState>) -> Result<Value, String> {
    let st = state.inner().clone();
    if !st.has_credentials() {
        return Err("请先在设置里填写 App ID 与私钥".into());
    }
    let (app_id, key, device_json) = endpoint_parts(&st);

    let anon = {
        let ep = Endpoint {
            http: &st.http,
            key: &key,
            app_id: &app_id,
            device: &device_json,
            device_id: &st.device.device_id,
        };
        auth::anonymous(&ep).await?
    };

    let ticket = {
        let ep = Endpoint {
            http: &st.http,
            key: &key,
            app_id: &app_id,
            device: &device_json,
            device_id: &st.device.device_id,
        };
        auth::qr_create(&ep, &anon).await?
    };

    let qr_url = ticket.qr_url.clone();
    let uni_key = ticket.uni_key.clone();
    let qr_url_return = ticket.qr_url;
    let uni_key_return = ticket.uni_key;
    let app_task = app.clone();
    let st_task = st.clone();
    upload::emit_log(&app, "info", "已生成登录二维码，等待扫码", Some("auth".into()));

    tauri::async_runtime::spawn(async move {
        for attempt in 0..96 {
            tokio::time::sleep(Duration::from_millis(2500)).await;
            let ep = Endpoint {
                http: &st_task.http,
                key: &key,
                app_id: &app_id,
                device: &device_json,
                device_id: &st_task.device.device_id,
            };
            match auth::qr_poll(&ep, &anon, &uni_key).await {
                Ok(outcome) => {
                    if attempt % 4 == 0 || outcome.status == 803 || outcome.status == 800 {
                        let _ = app_task.emit(
                            "auth://status",
                            json!({ "status": outcome.status, "message": outcome.message, "qrUrl": qr_url }),
                        );
                    }
                    if outcome.status == 803 {
                        if let Some(token) = outcome.token {
                            st_task.set_token(token);
                            let ep2 = Endpoint {
                                http: &st_task.http,
                                key: &key,
                                app_id: &app_id,
                                device: &device_json,
                                device_id: &st_task.device.device_id,
                            };
                            if let Some(profile) = st_task.access_token() {
                                if let Ok(p) = auth::profile(&ep2, &profile).await {
                                    if !p.is_null() {
                                        *st_task.profile.lock().unwrap() = Some(p);
                                    }
                                }
                            }
                            let _ = app_task.emit(
                                "auth://status",
                                json!({ "status": 803, "message": "登录成功", "qrUrl": qr_url }),
                            );
                            upload::emit_log(&app_task, "success", "登录成功", Some("auth".into()));
                        }
                        return;
                    }
                    if outcome.status == 800 {
                        upload::emit_log(&app_task, "warn", "二维码已过期", Some("auth".into()));
                        return;
                    }
                }
                Err(err) => {
                    upload::emit_log(&app_task, "warn", format!("轮询登录状态出错：{err}"), Some("auth".into()));
                }
            }
        }
        let _ = app_task.emit(
            "auth://status",
            json!({ "status": 800, "message": "二维码已过期，请重新获取", "qrUrl": qr_url }),
        );
    });

    Ok(json!({ "qrUrl": qr_url_return, "uniKey": uni_key_return }))
}

#[tauri::command]
pub async fn logout(state: State<'_, SharedState>) -> Result<(), String> {
    let st = state.inner().clone();
    st.clear_token();
    Ok(())
}

#[tauri::command]
pub async fn get_history(state: State<'_, SharedState>) -> Result<Vec<history::HistoryItem>, String> {
    let st = state.inner().clone();
    Ok(history::list(&st.app_dir))
}

#[tauri::command]
pub async fn clear_history(state: State<'_, SharedState>) -> Result<(), String> {
    let st = state.inner().clone();
    history::clear(&st.app_dir);
    Ok(())
}

const AUDIO_EXT: [&str; 11] = ["mp3", "flac", "m4a", "aac", "wav", "ogg", "opus", "aiff", "aif", "wma", "mp4"];

#[tauri::command]
pub async fn pick_files(app: AppHandle) -> Result<Vec<String>, String> {
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle
            .dialog()
            .file()
            .set_title("选择要上传的文件")
            .add_filter("音频文件", &AUDIO_EXT)
            .blocking_pick_files()
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(picked
        .unwrap_or_default()
        .into_iter()
        .filter_map(|f| f.into_path().ok())
        .map(|p| p.to_string_lossy().to_string())
        .collect())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Vec<String>, String> {
    let handle = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        handle.dialog().file().set_title("选择包含音乐的文件夹").blocking_pick_folder()
    })
    .await
    .map_err(|e| e.to_string())?;
    let Some(dir) = picked.and_then(|f| f.into_path().ok()) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    collect_audio(&dir, &mut out, 0);
    out.sort();
    Ok(out)
}

fn collect_audio(dir: &Path, out: &mut Vec<String>, depth: usize) {
    if depth > 8 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path: PathBuf = entry.path();
        if path.is_dir() {
            collect_audio(&path, out, depth + 1);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if AUDIO_EXT.contains(&ext.to_ascii_lowercase().as_str()) {
                out.push(path.to_string_lossy().to_string());
            }
        }
    }
}

#[tauri::command]
pub async fn upload_files(
    app: AppHandle,
    state: State<'_, SharedState>,
    files: Vec<String>,
) -> Result<(), String> {
    let st = state.inner().clone();
    if !st.busy.load(Ordering::SeqCst) {
        upload::spawn(app, st, files);
    }
    Ok(())
}

#[tauri::command]
pub async fn cancel_uploads(state: State<'_, SharedState>) -> Result<(), String> {
    let st = state.inner().clone();
    st.cancel.store(true, Ordering::SeqCst);
    Ok(())
}
