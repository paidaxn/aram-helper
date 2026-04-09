<script setup lang="ts">
import { ref } from "vue";

// 连接状态
const status = ref<"disconnected" | "connecting" | "connected">("disconnected");
const errorMsg = ref("");

// 模拟数据（后续从 Rust 后端获取）
const players = ref<
  {
    championId: number;
    teamId: number;
    damage: number;
    kills: number;
    deaths: number;
    assists: number;
  }[]
>([]);
</script>

<template>
  <div class="app">
    <!-- 等待连接 -->
    <div v-if="status === 'disconnected'" class="status-page">
      <div class="logo">🎮</div>
      <h1>ARAM 红包局助手</h1>
      <p class="hint">等待英雄联盟客户端启动...</p>
      <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
    </div>

    <!-- 连接中 -->
    <div v-else-if="status === 'connecting'" class="status-page">
      <div class="logo spinning">⚡</div>
      <h1>正在连接</h1>
      <p class="hint">已检测到客户端，连接中...</p>
    </div>

    <!-- 已连接，显示排名 -->
    <div v-else class="ranking-page">
      <h2>伤害排名</h2>
      <p class="sub">等待游戏结束后自动显示结果</p>

      <div v-if="players.length" class="player-list">
        <div
          v-for="(p, i) in players"
          :key="i"
          class="player-card"
          :class="{ lowest: i === players.length - 1 }"
        >
          <span class="rank">{{ i + 1 }}</span>
          <span class="champion">英雄 {{ p.championId }}</span>
          <span class="damage">{{ p.damage.toLocaleString() }}</span>
          <span class="kda">{{ p.kills }}/{{ p.deaths }}/{{ p.assists }}</span>
          <span v-if="i === players.length - 1" class="badge">💸 发红包</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
:root {
  --bg: #0f0f1a;
  --card: #1a1a2e;
  --text: #e0e0e0;
  --accent: #6366f1;
  --danger: #ef4444;
  --muted: #666;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  background: var(--bg);
  color: var(--text);
  font-family: -apple-system, "PingFang SC", "Microsoft YaHei", sans-serif;
}

.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 24px;
}

.status-page {
  text-align: center;
}

.logo {
  font-size: 64px;
  margin-bottom: 24px;
}

.spinning {
  animation: spin 2s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

h1 {
  font-size: 28px;
  font-weight: 700;
  margin-bottom: 12px;
}

.hint {
  color: var(--muted);
  font-size: 16px;
}

.error {
  color: var(--danger);
  margin-top: 12px;
  font-size: 14px;
}

.ranking-page {
  width: 100%;
  max-width: 500px;
}

h2 {
  font-size: 24px;
  margin-bottom: 8px;
}

.sub {
  color: var(--muted);
  font-size: 14px;
  margin-bottom: 24px;
}

.player-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.player-card {
  display: flex;
  align-items: center;
  gap: 12px;
  background: var(--card);
  padding: 14px 18px;
  border-radius: 12px;
  font-size: 15px;
}

.player-card.lowest {
  border: 2px solid var(--danger);
  background: rgba(239, 68, 68, 0.1);
}

.rank {
  font-weight: 700;
  width: 24px;
  color: var(--accent);
}

.champion {
  flex: 1;
}

.damage {
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.kda {
  color: var(--muted);
  font-size: 13px;
  width: 80px;
  text-align: right;
}

.badge {
  font-size: 13px;
}
</style>
