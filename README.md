# 云盘上传器 · NCM Cloud Drive

一个把本地音乐批量传进**自己的网易云音乐云盘**的桌面应用。基于网易云音乐开放平台 API + NOS 分块上传，走 1 MiB 分片与秒传校验，实时显示速度与进度。

- **技术栈**：Tauri 2（Rust）+ Vue 3 + Vite + TailwindCSS + Pinia + shadcn 风格组件
- **平台**：macOS（Apple Silicon / Intel）、Windows x64
- **构建**：GitHub Actions，**只有推送 `v*` tag 才会触发编译并发布 Release**

## 功能

| 页面 | 内容 |
| --- | --- |
| 登录 | 首次运行填写开放平台 App ID + RSA 私钥；之后用网易云音乐 App 扫码登录（二维码状态实时刷新：待扫 / 已扫待确认 / 成功 / 过期） |
| 上传工作台 | 选择文件或整目录递归导入；队列逐文件上传，显示百分比、实时速度、剩余时间；上传历史含 fileId / taskId / 秒传标记 / 平均速度；云盘占用估算 |
| 运行日志 | 全链路日志流（分级、可按级别筛选、自动滚动），上传细节一目了然 |

## 它是怎么工作的

1. 公共参数（appId / timestamp / device / bizContent …）去空值后按 key 升序拼接，**RSA-SHA256（PKCS#1 v1.5）** 签名，`signType=RSA_SHA256`。
2. 匿名 token → `qrcodekey/get` 取二维码 → 每 2.5s 轮询 `device/login/qrcode/get`（801 待扫 / 803 成功 / 800 过期），成功后落盘 token。
3. 上传：`cloud/file/upload/check` 先算 MD5 查秒传（`fileStatus` 0/1 直接命中）→ `whale/nos/upload/token` 换 NOS 凭证 → LBS 拿上传节点 → 1 MiB 分片串行 POST（`context` 链式依赖，天然无法并发）→ `cloud/file/upload` 提交 → `cloud/file/upload/status` 轮询导入完成。
4. 速度由客户端按分片时间戳实时测算（服务端进度粒度只有 1 MiB）。

实测（本机 200Mbps 上行、单连接）：稳态 **2.0–2.9 MB/s**，首块冷启动约 0.9 MB/s。

## 目录

```
src/                  Vue 前端（pages / stores / components）
src-tauri/src/api/    公共参数与签名、登录、云盘、NOS 分块
src-tauri/src/engine/ 上传队列与进度事件
src-tauri/src/store/  上传历史（本地 JSON）
.github/workflows/    tag 触发的矩阵构建
```

## 本地开发

```bash
npm install
npm run tauri dev      # 开发模式
npm run tauri build    # 本机打包
```

## 发布

```bash
git tag v0.1.0
git push origin v0.1.0
```

推送 tag 后 CI 会并行构建 macOS(arm64/x64) 与 Windows(x64)，产物自动挂到同名 Release。**推送普通提交不会触发任何编译。**

## 安全

- 应用**不内置任何密钥**；App ID 与私钥保存在本机应用数据目录（`credentials.json`，权限 `0600`），只用于本地签名请求。
- 私钥一旦泄露（贴到聊天、提交进仓库），必须去开放平台重置。
- token 与设备指纹（deviceId）绑定：换设备或改动 device 字段后必须重新扫码。

## 已知限制

- 官方**没有提供云盘列表接口**（`private/cloud/song/list/get` 返回 `300 应用未授权`），因此第二页是「上传工作台」而非云盘文件列表；云盘内容请去 App / 网页端查看。
- 同理没有删除云盘文件的能力。
- 分片 context 链式依赖，单文件只能串行上传；多文件目前也是逐文件排队（避免触发风控）。
