# 发版指南

## 首次设置（一次性）

### 1. 生成签名密钥对（用于自动更新）

在 Windows 上的项目根目录运行：

```bash
npx tauri signer generate -w ~/.tauri/aram-helper.key
```

会输出一个 **公钥**（pubkey），形如：

```
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6...
```

### 2. 把公钥填到 tauri.conf.json

打开 `src-tauri/tauri.conf.json`，找到：

```json
"updater": {
  "active": true,
  "endpoints": ["..."],
  "dialog": false,
  "pubkey": ""    ← 这里
}
```

把生成的公钥粘贴到 `pubkey` 字段。

### 3. 私钥保管

私钥默认在 `~/.tauri/aram-helper.key`，**绝不能泄露**。备份到密码管理器（1Password、Bitwarden 等）。

### 4. 设置环境变量（每次发版需要）

```bash
# Windows PowerShell
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $HOME\.tauri\aram-helper.key -Raw)
# 如果生成时设了密码，再加：
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "你的密码"
```

---

## 每次发版流程

### Step 1 · 升版本号

修改两处版本号（保持一致）：

- `package.json` → `"version": "0.1.1"`
- `src-tauri/tauri.conf.json` → `"version": "0.1.1"`
- `src-tauri/Cargo.toml` → `version = "0.1.1"`

### Step 2 · 提交并推送

```bash
git add -A
git commit -m "release: v0.1.1"
git push origin master
```

### Step 3 · 在 Windows 上构建

```bash
cd E:\repo\aram-helper
git pull
$env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content $HOME\.tauri\aram-helper.key -Raw)
npm run tauri build
```

构建产物：

- `src-tauri/target/release/aram-helper.exe` — 主程序
- `src-tauri/target/release/bundle/nsis/aram-helper_0.1.1_x64-setup.exe` — NSIS 安装包
- `src-tauri/target/release/bundle/nsis/aram-helper_0.1.1_x64-setup.exe.sig` — **签名文件**（自动更新关键）

### Step 4 · 创建 GitHub Release

打开 https://github.com/paidaxn/aram-helper/releases/new

- **Tag**：`v0.1.1`
- **Title**：`v0.1.1`
- **Description**：写更新内容
- **附件上传**（必须包含这两个）：
  - `aram-helper_0.1.1_x64-setup.exe`（安装包）
  - `aram-helper_0.1.1_x64-setup.exe.sig`（签名）
  - `latest.json`（更新清单，见 Step 5）

### Step 5 · 创建 latest.json（自动更新清单）

新建一个 `latest.json` 文件，内容：

```json
{
  "version": "0.1.1",
  "notes": "本次更新内容...",
  "pub_date": "2026-04-13T12:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "<复制 .sig 文件的全部内容>",
      "url": "https://github.com/paidaxn/aram-helper/releases/download/v0.1.1/aram-helper_0.1.1_x64-setup.exe"
    }
  }
}
```

`signature` 字段的值就是 `.sig` 文件里的全部文本（用记事本打开复制）。

把 `latest.json` 也作为附件上传到 Release。

### Step 6 · 发布

点 **Publish release**。

完成后：
- 官网下载按钮自动指向新版（通过 GitHub API 拉取）
- 已安装的旧版用户启动 app → 检查到更新 → 弹窗提示 → 一键更新

---

## 验证发版成功

1. 打开 https://daguagua.top — 下载按钮指向新版
2. 老版本启动 → 顶部出现紫色更新条 "发现新版本 v0.1.1"
3. 点击"立即更新" → 下载 → 自动重启 → 已是新版本

---

## 常见问题

**Q：第一次发版（v0.1.0）需要 latest.json 吗？**
A：不需要。第一版没有"前一版"可以更新到它。从 v0.1.0 起做就行。

**Q：忘了上传 .sig 文件怎么办？**
A：自动更新会失败（签名验证失败）。必须 .sig + .exe + latest.json 都齐。

**Q：私钥丢了？**
A：发新公钥重发整个 app（用户需要重新下载）。所以一定要备份私钥。

**Q：能用 GitHub Actions 自动化整个流程吗？**
A：可以。把私钥存到 GitHub Secrets，写 workflow 在打 tag 时自动构建发布。后续可以做。
