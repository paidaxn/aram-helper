# LOL 红包局助手 · 官网

静态网站，分享给朋友下载应用。

## 部署到 Vercel（推荐）

1. 访问 [vercel.com](https://vercel.com)，用 GitHub 账号登录
2. 点 **"Add New..." → "Project"**
3. 选择 `aram-helper` 仓库
4. 在 **Root Directory** 里填写 `website`
5. Framework Preset 选 **"Other"**（静态站，无需构建）
6. 点 **Deploy**

部署完成后会得到一个 `xxx.vercel.app` 的域名，分享给朋友即可。

## 绑定自定义域名（可选）

在 Vercel 项目设置 → Domains 添加你的域名，按提示配置 DNS。

推荐：
- `aram.yourname.com`
- 或买个短域名如 `xxx.xyz` 年费几块钱

## 部署到其他平台

同样支持：
- **Cloudflare Pages**：Root directory 填 `website`，Build 命令留空
- **Netlify**：Publish directory 填 `website`
- **GitHub Pages**：可用 Actions 自动部署 `website/` 目录

## 发版流程

1. 在 Windows 上 `npm run tauri build` 生成安装包
2. 推送 git tag：`git tag v0.1.1 && git push origin v0.1.1`
3. 在 GitHub Releases 页面手动创建 release，上传 `.exe` / `.msi` 安装包
4. 官网 `app.js` 会自动拉取最新 release，下载按钮自动指向新版本

**无需重新部署网站**，每次刷新都会从 GitHub API 拉取最新版本。

## 本地预览

```bash
cd website
python3 -m http.server 8000
# 浏览器打开 http://localhost:8000
```

## 文件说明

- `index.html` — 主页面
- `styles.css` — 样式（电竞紫金主题）
- `app.js` — JS 脚本，动态从 GitHub API 获取最新版
- `icon.png` — 应用图标
- `vercel.json` — Vercel 部署配置
