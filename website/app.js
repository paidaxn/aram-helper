// ────── 平台检测 + 非 Windows 体验优化 ──────
(function detectPlatform() {
  const ua = navigator.userAgent;
  const isMobile = /Mobile|Android|iP(hone|od|ad)/i.test(ua);
  const isWindows = /Windows/i.test(ua);
  const isMac = /Macintosh|Mac OS X/i.test(ua) && !isMobile;
  const isLinux = /Linux/i.test(ua) && !/Android/i.test(ua);

  const note = document.getElementById("platform-note");
  const banner = document.getElementById("mobile-banner");

  if (isMobile) {
    // 移动端：顶部横幅 + Hero 加醒目提示
    if (banner) banner.style.display = "block";
    document.body.classList.add("is-mobile");
    if (note) {
      note.style.display = "inline-block";
      note.textContent = "💻 本软件仅支持 Windows 电脑，请用电脑访问本站下载";
    }

    // 复制链接按钮
    const copyBtn = document.getElementById("copy-link-btn");
    if (copyBtn) {
      copyBtn.addEventListener("click", async () => {
        try {
          await navigator.clipboard.writeText("https://daguagua.top");
          copyBtn.textContent = "已复制 ✓";
          setTimeout(() => (copyBtn.textContent = "复制链接"), 2000);
        } catch {
          copyBtn.textContent = "复制失败";
        }
      });
    }
  } else if (isMac || isLinux) {
    // Mac/Linux：在 Hero meta 下方加一行平台说明
    if (note) {
      note.style.display = "inline-block";
      const os = isMac ? "Mac" : "Linux";
      note.textContent = `⚠ 检测到你使用 ${os}，本软件目前仅支持 Windows 10 / 11`;
    }
  }
})();

// ────── 事件追踪（钩子） ──────
function trackEvent(name, props = {}) {
  try {
    // Microsoft Clarity 自定义事件
    if (typeof window.clarity === "function") {
      window.clarity("event", name);
      // 额外上报属性作为标签（Clarity 支持）
      for (const [k, v] of Object.entries(props)) {
        if (v != null && v !== "") {
          window.clarity("set", k, String(v));
        }
      }
    }
    // 预留通用钩子
    if (typeof window.__track === "function") {
      window.__track(name, props);
    }
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
