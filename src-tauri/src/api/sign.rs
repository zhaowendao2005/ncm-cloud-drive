use std::collections::BTreeMap;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use rsa::{Pkcs1v15Sign, RsaPrivateKey};
use sha2::{Digest, Sha256};

/// 与开放平台约定一致：去掉 sign 与空值 → 按 key 升序 → `k=v` 用 `&` 连接。
pub fn canonical(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .filter(|(k, v)| k.as_str() != "sign" && !v.is_empty())
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// RSA-SHA256（PKCS#1 v1.5）签名，返回 base64。
pub fn sign(params: &BTreeMap<String, String>, key: &RsaPrivateKey) -> Result<String, String> {
    let msg = canonical(params);
    let mut hasher = Sha256::new();
    hasher.update(msg.as_bytes());
    let digest = hasher.finalize();
    let sig = key
        .sign(Pkcs1v15Sign::new::<Sha256>(), &digest)
        .map_err(|e| format!("签名失败：{e}"))?;
    Ok(B64.encode(sig))
}
