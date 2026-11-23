use serde_json::{json, Value};

use super::{code, message, Endpoint};
use crate::state::TokenSet;

pub struct QrTicket {
    pub qr_url: String,
    pub uni_key: String,
}

pub struct PollOutcome {
    pub status: i64,
    pub message: String,
    pub token: Option<TokenSet>,
}

/// 匿名 token：扫码接口本身需要一个 accessToken 参与签名。
pub async fn anonymous(ep: &Endpoint<'_>) -> Result<String, String> {
    let resp = ep
        .post("/openapi/music/basic/oauth2/login/anonymous", &json!({ "clientId": ep.app_id }), None)
        .await?;
    resp.pointer("/data/accessToken")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("匿名登录失败：{}", message(&resp)))
}

pub async fn qr_create(ep: &Endpoint<'_>, anon: &str) -> Result<QrTicket, String> {
    let resp = ep
        .get(
            "/openapi/music/basic/user/oauth2/qrcodekey/get/v2",
            &json!({ "type": 2, "expiredKey": "300" }),
            Some(anon),
        )
        .await?;
    if code(&resp) != 200 {
        return Err(format!("获取二维码失败：{}", message(&resp)));
    }
    Ok(QrTicket {
        qr_url: resp.pointer("/data/qrCodeUrl").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        uni_key: resp.pointer("/data/uniKey").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
    })
}

pub async fn qr_poll(ep: &Endpoint<'_>, anon: &str, uni_key: &str) -> Result<PollOutcome, String> {
    let resp = ep
        .get(
            "/openapi/music/basic/oauth2/device/login/qrcode/get",
            &json!({ "key": uni_key, "clientId": ep.app_id }),
            Some(anon),
        )
        .await?;
    if code(&resp) != 200 {
        return Err(format!("轮询登录状态失败：{}", message(&resp)));
    }
    let status = resp.pointer("/data/status").and_then(|v| v.as_i64()).unwrap_or(0);
    let msg = resp.pointer("/data/msg").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let token = resp
        .pointer("/data/accessToken/accessToken")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && *s != "null")
        .map(|access| TokenSet {
            access_token: access.to_string(),
            refresh_token: resp
                .pointer("/data/accessToken/refreshToken")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            expire_time: resp.pointer("/data/accessToken/expireTime").and_then(|v| v.as_i64()).unwrap_or(86400),
            device_id: ep.device_id().to_string(),
            obtained_at: chrono::Utc::now().timestamp(),
        });
    Ok(PollOutcome { status, message: msg, token })
}

pub async fn profile(ep: &Endpoint<'_>, token: &str) -> Result<Value, String> {
    let resp = ep
        .get("/openapi/music/basic/user/profile/get/v2", &json!({}), Some(token))
        .await?;
    Ok(resp.get("data").cloned().unwrap_or(Value::Null))
}
