use serde_json::Value;

use super::urlencode;

const LBS_HOST: &str = "http://wanproxy.127.net/lbs";

/// 从 LBS 拉取上传节点，返回形如 ["http://nosup-hz1.127.net", ...] 的基址列表。
pub async fn endpoints(http: &reqwest::Client, bucket: &str) -> Result<Vec<String>, String> {
    let url = format!("{LBS_HOST}?version=1.0&bucketname={}", urlencode(bucket));
    let resp = http.get(url).send().await.map_err(|e| format!("LBS 查询失败：{e}"))?;
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).map_err(|_| format!("LBS 响应异常：{}", &text.chars().take(160).collect::<String>()))?;
    let mut out = Vec::new();
    if let Some(arr) = v.get("upload").and_then(|u| u.as_array()) {
        for item in arr {
            let base = match item {
                Value::String(s) => s.clone(),
                Value::Object(o) => o
                    .get("host")
                    .or_else(|| o.get("ip"))
                    .and_then(|h| h.as_str())
                    .map(|h| if h.starts_with("http") { h.to_string() } else { format!("http://{h}") })
                    .unwrap_or_default(),
                _ => String::new(),
            };
            let base = base.trim_end_matches('/').to_string();
            if !base.is_empty() && !out.contains(&base) {
                out.push(base);
            }
        }
    }
    if out.is_empty() {
        return Err("LBS 未返回可用上传节点".into());
    }
    Ok(out)
}

/// 上传单个分片。返回 (context, 服务端确认的 offset)。
pub async fn upload_chunk(
    http: &reqwest::Client,
    node: &str,
    bucket: &str,
    nos_key: &str,
    nos_token: &str,
    offset: u64,
    complete: u8,
    context: Option<&str>,
    body: Vec<u8>,
) -> Result<(Option<String>, u64), String> {
    let mut url = format!(
        "{}/{}/{}?offset={}&complete={}&version=1.0",
        node,
        bucket,
        urlencode(nos_key),
        offset,
        complete
    );
    if let Some(c) = context {
        if !c.is_empty() {
            url.push_str(&format!("&context={}", urlencode(c)));
        }
    }
    let body_len = body.len() as u64;
    let resp = http
        .post(url)
        .header("x-nos-token", nos_token)
        .header("Content-Type", "application/octet-stream")
        .body(body)
        .send()
        .await
        .map_err(|e| format!("分片请求失败：{e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("分片上传 HTTP {}：{}", status.as_u16(), text.chars().take(160).collect::<String>()));
    }
    let v: Value = serde_json::from_str(&text).map_err(|_| format!("分片响应异常：{}", text.chars().take(160).collect::<String>()))?;
    let next_ctx = v.get("context").and_then(|c| c.as_str()).map(|s| s.to_string());
    let next_offset = v.get("offset").and_then(|o| o.as_u64()).unwrap_or(offset + body_len);
    Ok((next_ctx, next_offset))
}
