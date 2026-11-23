pub mod auth;
pub mod cloud;
pub mod nos;
pub mod sign;

use std::collections::BTreeMap;
use std::time::Duration;

use rsa::RsaPrivateKey;
use serde_json::Value;

use crate::state::BASE;

pub struct Endpoint<'a> {
    pub http: &'a reqwest::Client,
    pub key: &'a RsaPrivateKey,
    pub app_id: &'a str,
    pub device: &'a str,
    pub device_id: &'a str,
}

impl Endpoint<'_> {
    pub fn device_id(&self) -> &str {
        self.device_id
    }

    /// 组装公共参数 + 签名，返回 (query string, 签名原文)。
    fn build(&self, biz: &Value, token: Option<&str>) -> Result<String, String> {
        let mut p: BTreeMap<String, String> = BTreeMap::new();
        p.insert("appId".into(), self.app_id.to_string());
        p.insert("signType".into(), "RSA_SHA256".into());
        p.insert("timestamp".into(), chrono::Utc::now().timestamp_millis().to_string());
        p.insert("device".into(), self.device.to_string());
        p.insert("bizContent".into(), serde_json::to_string(biz).unwrap_or_else(|_| "{}".into()));
        if let Some(t) = token {
            if !t.is_empty() {
                p.insert("accessToken".into(), t.to_string());
            }
        }
        let sig = sign::sign(&p, self.key)?;
        p.insert("sign".into(), sig);
        Ok(p.iter().map(|(k, v)| format!("{}={}", urlencode(k), urlencode(v))).collect::<Vec<_>>().join("&"))
    }

    pub async fn get(&self, path: &str, biz: &Value, token: Option<&str>) -> Result<Value, String> {
        self.request(path, biz, token, false).await
    }

    pub async fn post(&self, path: &str, biz: &Value, token: Option<&str>) -> Result<Value, String> {
        self.request(path, biz, token, true).await
    }

    async fn request(&self, path: &str, biz: &Value, token: Option<&str>, post: bool) -> Result<Value, String> {
        let qs = self.build(biz, token)?;
        let url = format!("{BASE}{path}");
        let mut last = String::from("网络请求失败");
        for attempt in 0..4u32 {
            let req = if post {
                self.http
                    .post(&url)
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(qs.clone())
            } else {
                self.http.get(format!("{url}?{qs}"))
            };
            match req.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    match serde_json::from_str::<Value>(&text) {
                        Ok(v) => return Ok(v),
                        Err(_) => {
                            last = format!("响应不是 JSON（{}）：{}", status, truncate(&text, 180));
                        }
                    }
                }
                Err(e) => last = format!("网络错误：{e}"),
            }
            tokio::time::sleep(Duration::from_millis(700 * (attempt as u64 + 1))).await;
        }
        Err(last)
    }
}

pub(crate) fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

pub fn code(v: &Value) -> i64 {
    v.get("code").and_then(|c| c.as_i64()).unwrap_or(-1)
}

pub fn message(v: &Value) -> String {
    v.get("message").or_else(|| v.get("msg")).and_then(|m| m.as_str()).unwrap_or("").to_string()
}

/// 1406 = 当前设备未授权（token 与 deviceId 不匹配，只能重新扫码）。
pub fn is_device_mismatch(v: &Value) -> bool {
    code(v) == 1406 || message(v).contains("设备未授权")
}
