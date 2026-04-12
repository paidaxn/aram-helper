// ────── 事件追踪（钩子，后续接入分析服务时只需实现 window.__track） ──────
function trackEvent(name, props = {}) {
  try {
    if (typeof window.__track === "function") {
      window.__track(name, props);
    }
    // 调试日志（上线后可移除）
    if (window.location.hostname === "localhost") {
      console.log("[track]", name, props);
    }
  } catch (e) {}
}

function bindTracking() {
  // 下载按钮点击
  document.querySelectorAll("[data-track='download']").forEach((el) => {
    el.addEventListener("click", () => {
      trackEvent("download_click", {
        source: el.dataset.source || "unknown",
        url: el.getAttribute("href") || "",
      });
    });
  });

  // 导航/其他链接点击
  document.querySelectorAll("[data-track='nav']").forEach((el) => {
    el.addEventListener("click", () => {
      trackEvent("nav_click", { target: el.dataset.source || el.textContent?.trim() });
    });
  });

  // FAQ 展开
  document.querySelectorAll(".faq-item").forEach((el) => {
    el.addEventListener("toggle", () => {
      if (el.open) {
        trackEvent("faq_open", {
          question: el.querySelector("summary span")?.textContent?.trim(),
        });
      }
    });
  });

  // 页面浏览
  trackEvent("page_view", { path: window.location.pathname });
}

document.addEventListener("DOMContentLoaded", bindTracking);

// ────── 滚动触发动画（Intersection Observer） ──────
(() => {
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) {
          e.target.classList.add("visible");
          io.unobserve(e.target);
        }
      }
    },
    { threshold: 0.15, rootMargin: "0px 0px -8% 0px" }
  );

  document.querySelectorAll(".reveal").forEach((el) => io.observe(el));
})();

// ────── 从 GitHub API 拉取最新 release ──────
const GITHUB_OWNER = "paidaxn";
const GITHUB_REPO = "aram-helper";

(async function loadLatestRelease() {
  try {
    const res = await fetch(
      `https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/latest`
    );
    if (!res.ok) return;
    const data = await res.json();

    const tag = (data.tag_name || "").replace(/^v/, "");
    if (tag) {
      const label = document.getElementById("version-label");
      if (label) label.textContent = `最新版 v${tag}`;
    }

    const assets = data.assets || [];
    const preferred =
      assets.find((a) => /setup.*\.exe$/i.test(a.name)) ||
      assets.find((a) => /\.msi$/i.test(a.name)) ||
      assets.find((a) => /\.exe$/i.test(a.name));

    if (preferred) {
      document.querySelectorAll("[data-track='download']").forEach((btn) => {
        btn.setAttribute("href", preferred.browser_download_url);
      });
    }
  } catch (e) {
    console.warn("Failed to fetch release:", e);
  }
})();
