// 从 GitHub API 拉取最新 release，动态更新版本号和下载链接
const GITHUB_OWNER = "paidaxn";
const GITHUB_REPO = "aram-helper";

async function loadLatestRelease() {
  try {
    const res = await fetch(
      `https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/latest`
    );
    if (!res.ok) return;
    const data = await res.json();

    // 更新版本号
    const version = (data.tag_name || "").replace(/^v/, "");
    if (version) {
      document.getElementById("version").textContent = version;
    }

    // 查找 Windows 安装包（优先 .exe 安装包，回退到 .msi）
    const assets = data.assets || [];
    const preferred =
      assets.find((a) => /setup.*\.exe$/i.test(a.name)) ||
      assets.find((a) => /\.msi$/i.test(a.name)) ||
      assets.find((a) => /\.exe$/i.test(a.name));

    if (preferred) {
      document.getElementById("download-btn").href = preferred.browser_download_url;
    }
  } catch (e) {
    // 接口失败时保持默认链接（指向 releases 页）
    console.warn("Failed to fetch latest release:", e);
  }
}

loadLatestRelease();
