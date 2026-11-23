use serde_json::{json, Value};

use super::{code, message, Endpoint};

/// 秒传检查：fileStatus 0/1 = 已存在可直接复用，2 = 需要真实上传。
pub async fn check(ep: &Endpoint<'_>, token: &str, name: &str, size: u64, md5: &str) -> Result<Value, String> {
    let resp = ep
        .post(
            "/openapi/music/cloud/file/upload/check",
            &json!({ "fileName": name, "fileSize": size, "fileMd5": md5 }),
            Some(token),
        )
        .await?;
    if super::is_device_mismatch(&resp) {
        return Err("登录设备已失效（token 与设备指纹不匹配），请重新扫码登录".into());
    }
    if code(&resp) != 200 {
        return Err(format!("秒传检查失败：{}", message(&resp)));
    }
    resp.pointer("/data/results/0")
        .cloned()
        .ok_or_else(|| format!("秒传检查返回异常：{resp}"))
}

pub async fn upload_token(
    ep: &Endpoint<'_>,
    token: &str,
    bucket: &str,
    name: &str,
    md5: &str,
    size: u64,
    biz_key: &str,
) -> Result<(String, Value), String> {
    let resp = ep
        .post(
            "/openapi/whale/nos/upload/token",
            &json!({
                "bucket": bucket,
                "fileName": name,
                "fileMd5": md5,
                "fileSize": size,
                "timeout": 3600,
                "bizKey": biz_key,
                "resType": 1
            }),
            Some(token),
        )
        .await?;
    if code(&resp) != 200 {
        return Err(format!("获取上传凭证失败：{}", message(&resp)));
    }
    let upload_token = resp.pointer("/data/token").and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let resource = resp.pointer("/data/resource").cloned().unwrap_or(Value::Null);
    if upload_token.is_empty() || resource.is_null() {
        return Err(format!("上传凭证字段缺失：{resp}"));
    }
    Ok((upload_token, resource))
}

pub async fn submit(
    ep: &Endpoint<'_>,
    token: &str,
    resource_id: &str,
    name: &str,
    size: u64,
    md5: &str,
    bucket: &str,
    nos_key: &str,
) -> Result<Value, String> {
    let resp = ep
        .post(
            "/openapi/music/cloud/file/upload",
            &json!({
                "resourceId": resource_id,
                "fileName": name,
                "fileSize": size,
                "fileMd5": md5,
                "whaleKey": format!("{bucket}/{nos_key}"),
                "nosKey": nos_key,
                "fileStatus": 2
            }),
            Some(token),
        )
        .await?;
    if code(&resp) != 200 {
        return Err(format!("提交导入任务失败：{}", message(&resp)));
    }
    Ok(resp.get("data").cloned().unwrap_or(Value::Null))
}

pub async fn task_status(ep: &Endpoint<'_>, token: &str, task_id: i64) -> Result<Value, String> {
    let resp = ep
        .post(
            "/openapi/music/cloud/file/upload/status",
            &json!({ "taskIds": [task_id] }),
            Some(token),
        )
        .await?;
    if code(&resp) != 200 {
        return Err(format!("查询导入状态失败：{}", message(&resp)));
    }
    Ok(resp.pointer("/data/taskList/0").cloned().unwrap_or(Value::Null))
}
