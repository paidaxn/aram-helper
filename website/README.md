# LOL 红包局助手 · 官网

静态网站，分享给朋友下载应用。

## 国内可访问的部署方案（按推荐顺序）

### 方案 1：Cloudflare Pages（推荐，免费）

国内 80% 能直接访问，且免费不限流量。

1. 访问 [pages.cloudflare.com](https://pages.cloudflare.com)，注册账号
2. **Create a project** → **Connect to Git** → 授权 GitHub
3. 选择 `aram-helper` 仓库
4. 构建配置：
   - **Framework preset**：None
   - **Build command**：留空
   - **Build output directory**：`website`
5. Deploy，拿到 `xxx.pages.dev` 域名

### 方案 2：腾讯云 EdgeOne Pages（国内最快，免费）

专为国内优化，访问速度最快。

1. 访问 [edgeone.cloud.tencent.com/pages](https://edgeone.cloud.tencent.com/pages)
2. 登录腾讯云账号（微信/QQ 一键登录）
3. 新建项目 → 导入 GitHub 仓库
4. 项目根目录填 `website`，构建命令留空
5. Deploy，拿到 `xxx.pages.tencentedge.com` 域名

### 方案 3：Gitee Pages（简单，免费）

1. 把代码推到 Gitee（你已有 `gitee.com/zhangshijieee/aram-helper`）
2. 仓库 → 服务 → Gitee Pages
3. 部署目录填 `website`，点启动
4. 拿到 `zhangshijieee.gitee.io/aram-helper` 域名

### 方案 4：阿里云 OSS 静态托管（最快，收费）

- 月费几元
- 需阿里云账号
- 适合后期有稳定流量时考虑

## 绑定自定义域名（可选）

买个便宜域名（`.xyz`/`.top` 每年几块到几十块），在部署平台的 Domain 设置里绑定。

推荐：
- `aram.你的名字.xyz`
- Cloudflare / 腾讯 DNSPod 免费解析

## 关于 Binary 下载（重要）

网站本身国内能访问了，但下载按钮指向 **GitHub Releases** —— 国内下载 GitHub 通常能通，但速度看运营商：

- **教育网 / 电信**：一般能下，速度几百 KB/s
- **移动网络**：可能较慢
- **部分地区**：可能完全下不了

**如果朋友反馈下载不了**，加一个 `ghproxy.cn` 镜像后备：

```javascript
// website/app.js 里，把下载链接改成:
const proxied = `https://ghproxy.cn/${preferred.browser_download_url}`;
```

## 发版流程

1. 在 Windows 上 `npm run tauri build` 生成安装包
2. 打 tag：`git tag v0.1.1 && git push origin v0.1.1`
3. GitHub → Releases → New release，选择 tag，上传 `.exe` / `.msi`
4. **官网不需要重新部署** —— 每次刷新都会从 GitHub API 拉取最新版

## 本地预览

```bash
cd website
python3 -m http.server 8000
# 浏览器打开 http://localhost:8000
```

## 文件说明

- `index.html` — 主页
- `styles.css` — 样式（电竞紫金主题）
- `app.js` — 动态从 GitHub API 获取最新版本
- `icon.png` — 应用图标
- `vercel.json` — Vercel 部署配置（备用）
