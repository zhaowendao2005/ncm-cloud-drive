//! 真实环境联调测试：用本机已有的开放平台凭据验证「签名 → 鉴权 → 秒传检查」链路。
//!
//! 依赖 `NCM_SECRETS`（默认 `$HOME/.ncm-secrets`）下的 `private_key.pem` 与 `tokens.json`。
//! 缺文件时直接跳过，保证 CI 上没有凭据也能跑。

use std::path::{Path, PathBuf};

use ncm_cloud_drive_lib::api::{auth, cloud, Endpoint};
use ncm_cloud_drive_lib::state::{parse_key, Device};

const APP_ID: &str = "b3010d0000000000a2dd0a8e41f88035";

fn secrets_dir() -> PathBuf {
    match std::env::var("NCM_SECRETS") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".ncm-secrets"),
    }
}

fn md5_of(path: &Path) -> String {
    use md5::{Digest, Md5};
    let mut f = std::fs::File::open(path).expect("open");
    let mut h = Md5::new();
    let mut buf = vec![0u8; 1 << 20];
    use std::io::Read;
    loop {
        let n = f.read(&mut buf).expect("read");
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    format!("{:x}", h.finalize())
}

#[tokio::test]
async fn live_sign_and_upload_check() {
    let dir = secrets_dir();
    let key_path = dir.join("private_key.pem");
    let token_path = dir.join("tokens.json");
    if !key_path.exists() || !token_path.exists() {
        eprintln!("跳过：没有找到 {:?} / {:?}", key_path, token_path);
        return;
    }

    let raw_key = std::fs::read_to_string(&key_path).unwrap();
    let key = parse_key(&raw_key).expect("私钥解析失败");
    let token_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&token_path).unwrap()).unwrap();
    let access_token = token_json["accessToken"].as_str().expect("accessToken").to_string();
    let device_id = token_json["deviceId"].as_str().unwrap_or_default().to_string();

    let device = Device {
        device_id: device_id.clone(),
        ..Default::default()
    };
    let device_json = device.to_json();
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("ncm-cloud-drive/0.1.0")
        .build()
        .unwrap();
    let ep = Endpoint {
        http: &http,
        key: &key,
        app_id: APP_ID,
        device: &device_json,
        device_id: &device.device_id,
    };

    // 1) 签名 + 鉴权：能拿到用户资料说明公共参数与 RSA 签名完全正确
    let profile = auth::profile(&ep, &access_token).await.expect("profile 请求失败");
    let nickname = profile["nickname"].as_str().unwrap_or("<none>");
    println!("profile ok: nickname={nickname}");

    // 2) 秒传检查：取一首已经传过的歌，应当直接命中 fileStatus=0
    let sample = PathBuf::from(
        "/Users/qihoo/Downloads/1872629025_朝闻道2222/默认收藏夹/mp3/久石让 - 人生的旋转木马.mp3",
    );
    if sample.exists() {
        let name = sample.file_name().unwrap().to_string_lossy().to_string();
        let size = std::fs::metadata(&sample).unwrap().len();
        let md5 = md5_of(&sample);
        let result = cloud::check(&ep, &access_token, &name, size, &md5)
            .await
            .expect("秒传检查失败");
        let status = result["fileStatus"].as_i64().unwrap_or(-1);
        println!("check ok: {name} fileStatus={status}");
        assert!(
            status == 0 || status == 1,
            "期望秒传命中（0/1），实际 {status}: {result}"
        );
    } else {
        println!("跳过秒传检查：样本文件不存在");
    }
}
