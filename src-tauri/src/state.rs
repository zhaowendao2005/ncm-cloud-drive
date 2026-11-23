use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use serde::{Deserialize, Serialize};

pub const BASE: &str = "https://openncm.music.163.com";
pub const CHUNK_SIZE: usize = 1024 * 1024;
pub const MAX_HISTORY: usize = 500;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Device {
    #[serde(rename = "deviceType")]
    pub device_type: String,
    pub os: String,
    #[serde(rename = "appVer")]
    pub app_ver: String,
    pub channel: String,
    pub model: String,
    pub brand: String,
    #[serde(rename = "osVer")]
    pub os_ver: String,
    #[serde(rename = "clientIp")]
    pub client_ip: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
}

impl Default for Device {
    /// 这些取值是开放平台公共参数校验通过的组合，除 deviceId 外不要随意修改，
    /// 否则网关会返回「公共参数校验失败，请检查os/channel/brand等是否符合约定」。
    fn default() -> Self {
        Self {
            device_type: "openapi".into(),
            os: "ncmcli".into(),
            app_ver: "0.1.7".into(),
            channel: "ncmcli".into(),
            model: "mac_cli".into(),
            brand: "ncmcli".into(),
            os_ver: "15.1.1".into(),
            client_ip: "127.0.0.1".into(),
            device_id: random_hex(16),
        }
    }
}

impl Device {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

pub fn random_hex(bytes: usize) -> String {
    use rand::RngCore;
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    buf.iter().map(|b| format!("{:02x}", b)).collect()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TokenSet {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expireTime")]
    pub expire_time: i64,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "obtainedAt")]
    #[serde(default)]
    pub obtained_at: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Credentials {
    #[serde(rename = "appId")]
    pub app_id: String,
    /// 裸 base64（不含 PEM 头尾）或完整 PEM 均可。
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

pub struct AppState {
    pub app_dir: PathBuf,
    pub device: Device,
    pub key: Mutex<RsaPrivateKey>,
    pub app_id: Mutex<String>,
    pub http: reqwest::Client,
    pub token: Mutex<Option<TokenSet>>,
    pub profile: Mutex<Option<serde_json::Value>>,
    pub cancel: AtomicBool,
    pub busy: AtomicBool,
}

impl AppState {
    pub fn new(app_dir: PathBuf) -> Self {
        let device = load_json::<Device>(&app_dir.join("device.json")).unwrap_or_default();
        let _ = save_json(&app_dir.join("device.json"), &device);

        let creds = load_json::<Credentials>(&app_dir.join("credentials.json"));
        let (app_id, key) = match creds.as_ref().and_then(|c| parse_key(&c.private_key).ok().map(|k| (c.app_id.clone(), k))) {
            Some((id, k)) => (id, Some(k)),
            None => (String::new(), None),
        };

        let token = load_json::<TokenSet>(&app_dir.join("tokens.json"));

        Self {
            app_dir,
            device,
            key: Mutex::new(key.unwrap_or_else(Self::placeholder_key)),
            app_id: Mutex::new(app_id),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("ncm-cloud-drive/0.1.0")
                .build()
                .expect("http client"),
            token: Mutex::new(token),
            profile: Mutex::new(None),
            cancel: AtomicBool::new(false),
            busy: AtomicBool::new(false),
        }
    }

    /// 尚未配置私钥时给一个占位 key，避免 Option 到处传播；调用前用 has_credentials 判断。
    fn placeholder_key() -> RsaPrivateKey {
        use rsa::traits::PublicKeyParts;
        let mut rng = rand::thread_rng();
        let k = RsaPrivateKey::new(&mut rng, 512).expect("placeholder rsa");
        let _ = k.n().bits();
        k
    }

    pub fn has_credentials(&self) -> bool {
        !self.app_id.lock().unwrap().is_empty()
            && load_json::<Credentials>(&self.app_dir.join("credentials.json")).is_some()
    }

    pub fn token(&self) -> Option<TokenSet> {
        self.token.lock().unwrap().clone()
    }

    /// token 有效期由 `obtainedAt + expireTime`（秒）决定，过期后只能重新扫码。
    pub fn token_valid(&self) -> bool {
        match self.token() {
            Some(t) => {
                let now = chrono::Utc::now().timestamp();
                t.obtained_at + t.expire_time > now + 60
            }
            None => false,
        }
    }

    pub fn access_token(&self) -> Option<String> {
        self.token().map(|t| t.access_token)
    }

    pub fn set_token(&self, t: TokenSet) {
        let _ = save_json(&self.app_dir.join("tokens.json"), &t);
        *self.token.lock().unwrap() = Some(t);
    }

    pub fn clear_token(&self) {
        let _ = std::fs::remove_file(self.app_dir.join("tokens.json"));
        *self.token.lock().unwrap() = None;
        *self.profile.lock().unwrap() = None;
    }
}

pub fn parse_key(raw: &str) -> Result<RsaPrivateKey, String> {
    let trimmed = raw.trim();
    if trimmed.starts_with("-----BEGIN") {
        return RsaPrivateKey::from_pkcs8_pem(trimmed)
            .or_else(|_| {
                use rsa::pkcs1::DecodeRsaPrivateKey;
                RsaPrivateKey::from_pkcs1_pem(trimmed)
            })
            .map_err(|e| format!("私钥解析失败：{e}"));
    }
    let body: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    let wrapped: String = body
        .as_bytes()
        .chunks(64)
        .map(|c| String::from_utf8_lossy(c).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let pem = format!("-----BEGIN PRIVATE KEY-----\n{wrapped}\n-----END PRIVATE KEY-----\n");
    RsaPrivateKey::from_pkcs8_pem(&pem).map_err(|e| format!("私钥解析失败：{e}"))
}

pub fn save_credentials(dir: &Path, app_id: &str, private_key: &str) -> Result<(), String> {
    parse_key(private_key)?;
    let c = Credentials {
        app_id: app_id.trim().to_string(),
        private_key: private_key.trim().to_string(),
    };
    save_json(&dir.join("credentials.json"), &c)?;
    restrict_permissions(&dir.join("credentials.json"));
    Ok(())
}

pub fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let data = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    let data = std::fs::read(path).ok()?;
    serde_json::from_slice(&data).ok()
}

#[cfg(unix)]
pub fn restrict_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
pub fn restrict_permissions(_path: &Path) {}

pub type SharedState = Arc<AppState>;
