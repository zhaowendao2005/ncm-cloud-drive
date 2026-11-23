use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::api::{cloud, nos, Endpoint};
use crate::state::{SharedState, CHUNK_SIZE};
use crate::store::history::{self, HistoryItem};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Progress {
    file: String,
    index: usize,
    total: usize,
    loaded: u64,
    total_bytes: u64,
    percent: f64,
    speed: f64,
    avg_speed: f64,
    eta: f64,
}

pub fn spawn(app: AppHandle, state: SharedState, files: Vec<String>) {
    tauri::async_runtime::spawn(async move {
        run(app, state, files).await;
    });
}

async fn run(app: AppHandle, st: SharedState, files: Vec<String>) {
    let token = match st.access_token() {
        Some(t) => t,
        None => {
            emit_log(&app, "error", "尚未登录，无法上传", None);
            return;
        }
    };
    if st.busy.swap(true, Ordering::SeqCst) {
        emit_log(&app, "warn", "已有上传任务在进行中", None);
        return;
    }
    st.cancel.store(false, Ordering::SeqCst);
    let _ = app.emit("upload://running", true);

    let device_json = st.device.to_json();
    let (app_id, key) = (st.app_id.lock().unwrap().clone(), st.key.lock().unwrap().clone());
    let total = files.len();
    for (index, file) in files.iter().enumerate() {
        if st.cancel.load(Ordering::SeqCst) {
            emit_log(&app, "warn", "已按用户请求停止队列", None);
            break;
        }
        let path = PathBuf::from(file);
        let ep = Endpoint {
            http: &st.http,
            key: &key,
            app_id: &app_id,
            device: &device_json,
            device_id: &st.device.device_id,
        };
        match upload_one(&app, &st, &ep, &token, &path, index, total).await {
            Ok(()) => {
                let _ = app.emit("upload://file-done", file.clone());
            }
            Err(err) => {
                emit_log(&app, "error", err.clone(), Some(file.clone()));
                let _ = app.emit("upload://file-error", file.clone());
            }
        }
    }

    st.busy.store(false, Ordering::SeqCst);
    let _ = app.emit("upload://running", false);
    emit_log(&app, "info", format!("队列结束，共 {} 个文件", total), None);
}

async fn upload_one(
    app: &AppHandle,
    st: &SharedState,
    ep: &Endpoint<'_>,
    token: &str,
    path: &Path,
    index: usize,
    total: usize,
) -> Result<(), String> {
    let name = file_name(path);
    let size = std::fs::metadata(path).map_err(|e| format!("读取文件失败：{e}"))?.len();
    if size == 0 {
        return Err(format!("{name} 是空文件"));
    }
    emit_log(app, "info", format!("开始处理 {name}（{}）", human(size)), Some(name.clone()));

    let md5 = md5_file(path).await?;
    emit_log(app, "info", format!("MD5 {md5}"), Some(name.clone()));

    let check = cloud::check(ep, token, &name, size, &md5).await?;
    let file_status = check.get("fileStatus").and_then(|v| v.as_i64()).unwrap_or(2);
    if file_status == 0 || file_status == 1 {
        let file_id = check.get("fileId").and_then(|v| v.as_i64()).unwrap_or(0);
        history::append(
            &st.app_dir,
            HistoryItem {
                file_name: name.clone(),
                file_size: size,
                file_md5: md5.clone(),
                file_id,
                task_id: 0,
                fast: true,
                status: "exists".into(),
                uploaded_at: now_ms(),
                duration_ms: 0,
                avg_speed: 0.0,
            },
        );
        emit_log(app, "success", format!("{name} 已在云盘，秒传命中"), Some(name.clone()));
        return Ok(());
    }

    let bucket = check
        .pointer("/nosUploadInfo/bucket")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "缺少 nosUploadInfo.bucket".to_string())?
        .to_string();
    let biz_key = check
        .pointer("/nosUploadInfo/bizKey")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let (nos_token, resource) = cloud::upload_token(ep, token, &bucket, &name, &md5, size, &biz_key).await?;
    let resource_id = json_scalar(&resource, "id");
    let nos_key = resource
        .get("nosKey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "上传凭证缺少 nosKey".to_string())?
        .to_string();
    let bucket = resource
        .get("bucket")
        .and_then(|v| v.as_str())
        .unwrap_or(&bucket)
        .to_string();

    let nodes = nos::endpoints(&st.http, &bucket).await?;
    emit_log(app, "info", format!("分配节点 {}（{} 个可用）", nodes[0], nodes.len()), Some(name.clone()));

    let mut file = std::fs::File::open(path).map_err(|e| format!("打开文件失败：{e}"))?;
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut offset: u64 = 0;
    let mut context: Option<String> = None;
    let mut node_idx = 0usize;
    let started = Instant::now();
    let mut last_at = started;
    let mut last_loaded: u64 = 0;

    while offset < size {
        if st.cancel.load(Ordering::SeqCst) {
            return Err("已取消".into());
        }
        let len = std::cmp::min(CHUNK_SIZE as u64, size - offset) as usize;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("定位文件失败：{e}"))?;
        file.read_exact(&mut buf[..len])
            .map_err(|e| format!("读取分片失败：{e}"))?;
        let complete = if offset + len as u64 >= size { 1u8 } else { 0u8 };

        let mut attempt = 0u32;
        loop {
            let chunk = buf[..len].to_vec();
            match nos::upload_chunk(
                &st.http,
                &nodes[node_idx],
                &bucket,
                &nos_key,
                &nos_token,
                offset,
                complete,
                context.as_deref(),
                chunk,
            )
            .await
            {
                Ok((ctx, next)) => {
                    context = ctx;
                    offset = if next > offset { next } else { offset + len as u64 };
                    break;
                }
                Err(err) => {
                    attempt += 1;
                    if attempt >= 3 {
                        return Err(format!("分片上传失败（已重试）：{err}"));
                    }
                    node_idx = (node_idx + 1) % nodes.len();
                    emit_log(
                        app,
                        "warn",
                        format!("{name} 分片重试 {attempt}/3，切换节点 {}", nodes[node_idx]),
                        Some(name.clone()),
                    );
                    tokio::time::sleep(Duration::from_millis(400)).await;
                }
            }
        }

        let now = Instant::now();
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        let recent = now.duration_since(last_at).as_secs_f64().max(0.001);
        let instant = (offset - last_loaded) as f64 / recent;
        let avg = offset as f64 / elapsed;
        last_at = now;
        last_loaded = offset;
        let eta = if avg > 1.0 { (size - offset) as f64 / avg } else { 0.0 };
        let _ = app.emit(
            "upload://progress",
            Progress {
                file: path.to_string_lossy().to_string(),
                index,
                total,
                loaded: offset,
                total_bytes: size,
                percent: if size > 0 { offset as f64 / size as f64 * 100.0 } else { 0.0 },
                speed: instant,
                avg_speed: avg,
                eta,
            },
        );
    }

    let duration_ms = started.elapsed().as_millis() as u64;
    let avg_speed = size as f64 / (duration_ms as f64 / 1000.0).max(0.001);

    let submit = cloud::submit(ep, token, &resource_id, &name, size, &md5, &bucket, &nos_key).await?;
    let task_id = submit.get("taskId").and_then(|v| v.as_i64()).unwrap_or(0);
    emit_log(
        app,
        "info",
        format!("{name} 数据已上传（{:.1} MB/s），等待服务端导入 taskId={task_id}", avg_speed / 1_048_576.0),
        Some(name.clone()),
    );

    let mut file_id = 0i64;
    for _ in 0..40 {
        if st.cancel.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
        match cloud::task_status(ep, token, task_id).await {
            Ok(t) => {
                let status = t.get("taskStatus").and_then(|v| v.as_i64()).unwrap_or(0);
                file_id = t.get("fileId").and_then(|v| v.as_i64()).unwrap_or(0);
                if status == 10 || file_id > 0 {
                    break;
                }
            }
            Err(err) => emit_log(app, "warn", format!("查询导入状态失败：{err}"), Some(name.clone())),
        }
    }

    history::append(
        &st.app_dir,
        HistoryItem {
            file_name: name.clone(),
            file_size: size,
            file_md5: md5.clone(),
            file_id,
            task_id,
            fast: false,
            status: if file_id > 0 { "done".into() } else { "pending".into() },
            uploaded_at: now_ms(),
            duration_ms,
            avg_speed,
        },
    );
    emit_log(
        app,
        "success",
        if file_id > 0 {
            format!("{name} 上传完成，fileId={file_id}")
        } else {
            format!("{name} 已提交，服务端仍在导入（taskId={task_id}）")
        },
        Some(name.clone()),
    );
    Ok(())
}

async fn md5_file(path: &Path) -> Result<String, String> {
    use md5::{Digest, Md5};
    let path = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let mut f = std::fs::File::open(&path).map_err(|e| format!("打开文件失败：{e}"))?;
        let mut hasher = Md5::new();
        let mut buf = vec![0u8; 1 << 20];
        loop {
            let n = f.read(&mut buf).map_err(|e| format!("读取文件失败：{e}"))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    })
    .await
    .map_err(|e| format!("哈希任务失败：{e}"))?
}

fn json_scalar(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

pub fn file_name(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

pub fn emit_log(app: &AppHandle, level: &str, message: impl Into<String>, scope: Option<String>) {
    let _ = app.emit(
        "upload://log",
        json!({
            "level": level,
            "message": message.into(),
            "time": now_ms(),
            "scope": scope
        }),
    );
}
