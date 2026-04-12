<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

// 应用状态
type AppStatus = "disconnected" | "connected" | "result";
const status = ref<AppStatus>("disconnected");
const summonerName = ref("");
const gamePhase = ref("");
const errorMsg = ref("");
const copied = ref(false);

// 玩家数据
interface Player {
  summonerName: string;
  championId: number;
  teamId: number;
  damage: number;
  kills: number;
  deaths: number;
  assists: number;
  isLowest: boolean;
}
const players = ref<Player[]>([]);

// 伤害最低的玩家
const loser = computed(() => players.value.find((p) => p.isLowest));

// 轮询定时器
let pollTimer: ReturnType<typeof setTimeout> | null = null;

// 清除定时器
function clearPoll() {
  if (pollTimer) {
    clearTimeout(pollTimer);
    pollTimer = null;
  }
}

// 轮询连接状态（每 5 秒）
async function pollConnection() {
  try {
    const name = await invoke<string>("check_connection");
    summonerName.value = name;
    errorMsg.value = "";
    status.value = "connected";
    pollGamePhase();
  } catch (e) {
    status.value = "disconnected";
    errorMsg.value = String(e);
    pollTimer = setTimeout(pollConnection, 5000);
  }
}

// 轮询游戏状态（每 3 秒）
async function pollGamePhase() {
  try {
    const phase = await invoke<string>("get_gameflow_phase");
    gamePhase.value = phase;

    if (phase === "EndOfGame" || phase === "WaitingForStats") {
      await fetchDamageRanking();
    } else {
      pollTimer = setTimeout(pollGamePhase, 3000);
    }
  } catch {
    // 连接丢失
    status.value = "disconnected";
    pollTimer = setTimeout(pollConnection, 5000);
  }
}

// 获取伤害排名
async function fetchDamageRanking() {
  try {
    const ranking = await invoke<Player[]>("get_damage_ranking");
    players.value = ranking;
    status.value = "result";
  } catch (e) {
    errorMsg.value = String(e);
    pollTimer = setTimeout(pollGamePhase, 3000);
  }
}

// 手动查看上一局（用于测试验证）
const loadingLastGame = ref(false);
async function viewLastGame() {
  loadingLastGame.value = true;
  errorMsg.value = "";
  clearPoll();
  try {
    const ranking = await invoke<Player[]>("get_damage_ranking");
    players.value = ranking;
    status.value = "result";
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loadingLastGame.value = false;
  }
}

// 复制结果到剪贴板
async function copyResult() {
  const lines = players.value.map(
    (p, i) =>
      `${i + 1}. ${p.summonerName}  ${p.damage.toLocaleString()} 伤害  ${p.kills}/${p.deaths}/${p.assists}${p.isLowest ? " 💸" : ""}`
  );
  const text = `🏆 ARAM 红包局结算\n${lines.join("\n")}\n\n${loser.value?.summonerName} 伤害最低，发红包！`;
  await navigator.clipboard.writeText(text);
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}

// 继续等待下一局
function waitNext() {
  status.value = "connected";
  gamePhase.value = "";
  players.value = [];
  pollGamePhase();
}

// 游戏阶段中文映射
function phaseText(phase: string): string {
  const map: Record<string, string> = {
    None: "大厅",
    Lobby: "房间中",
    Matchmaking: "匹配中",
    ReadyCheck: "确认中",
    ChampSelect: "选英雄中",
    GameStart: "游戏启动中",
    InProgress: "游戏进行中",
    WaitingForStats: "等待结算",
    EndOfGame: "对局结束",
  };
  return map[phase] || phase;
}

onMounted(() => {
  pollConnection();
});

onUnmounted(() => {
  clearPoll();
});
</script>

<template>
  <div class="app">
    <!-- 等待连接 -->
    <div v-if="status === 'disconnected'" class="status-page">
      <div class="logo">🎮</div>
      <h1>ARAM 红包局助手</h1>
      <p class="hint">等待英雄联盟客户端启动...</p>
      <div class="dots"><span /><span /><span /></div>
      <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
    </div>

    <!-- 已连接，等待游戏结束 -->
    <div v-else-if="status === 'connected'" class="status-page">
      <div class="logo pulse">⚡</div>
      <h1>已连接</h1>
      <p class="summoner">{{ summonerName }}</p>
      <p class="hint">
        {{ gamePhase ? `当前：${phaseText(gamePhase)}` : "等待游戏结束后自动显示结果" }}
      </p>
      <div class="dots"><span /><span /><span /></div>
      <button class="btn-last-game" @click="viewLastGame" :disabled="loadingLastGame">
        {{ loadingLastGame ? "加载中..." : "查看上一局" }}
      </button>
      <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
    </div>

    <!-- 对局结果 -->
    <div v-else class="result-page">
      <h2>🏆 本局红包局结算</h2>

      <div class="player-list">
        <div
          v-for="(p, i) in players"
          :key="i"
          class="player-card"
          :class="{ lowest: p.isLowest }"
        >
          <span class="rank">{{ i + 1 }}</span>
          <span class="name">{{ p.summonerName }}</span>
          <span class="damage">{{ p.damage.toLocaleString() }}</span>
          <span class="kda">{{ p.kills }}/{{ p.deaths }}/{{ p.assists }}</span>
          <span v-if="p.isLowest" class="badge">💸</span>
        </div>
      </div>

      <div v-if="loser" class="loser-banner">
        {{ loser.summonerName }} 伤害最低，发红包！
      </div>

      <div class="actions">
        <button class="btn primary" @click="copyResult">
          {{ copied ? "已复制 ✓" : "复制结果" }}
        </button>
        <button class="btn" @click="waitNext">继续等待</button>
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
  user-select: none;
}

.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 24px;
}

/* 状态页 */
.status-page {
  text-align: center;
}

.logo {
  font-size: 64px;
  margin-bottom: 24px;
}

.pulse {
  animation: pulse 2s ease-in-out infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.7; transform: scale(1.1); }
}

h1 {
  font-size: 28px;
  font-weight: 700;
  margin-bottom: 12px;
}

.summoner {
  color: var(--accent);
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 8px;
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

/* 加载动画 */
.dots {
  display: flex;
  justify-content: center;
  gap: 6px;
  margin-top: 24px;
}

.dots span {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--muted);
  animation: dot 1.4s ease-in-out infinite;
}

.dots span:nth-child(2) { animation-delay: 0.2s; }
.dots span:nth-child(3) { animation-delay: 0.4s; }

.btn-last-game {
  margin-top: 24px;
  padding: 10px 24px;
  border: 1px solid var(--muted);
  border-radius: 8px;
  background: transparent;
  color: var(--text);
  font-size: 14px;
  cursor: pointer;
}

.btn-last-game:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-last-game:not(:disabled):active {
  background: rgba(255, 255, 255, 0.05);
}

@keyframes dot {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1.2); }
}

/* 结果页 */
.result-page {
  width: 100%;
  max-width: 420px;
}

h2 {
  font-size: 22px;
  text-align: center;
  margin-bottom: 20px;
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
  padding: 14px 16px;
  border-radius: 12px;
  font-size: 15px;
  border: 2px solid transparent;
  transition: border-color 0.2s;
}

.player-card.lowest {
  border-color: var(--danger);
  background: rgba(239, 68, 68, 0.08);
}

.rank {
  font-weight: 700;
  width: 20px;
  color: var(--accent);
  flex-shrink: 0;
}

.name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.damage {
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
}

.kda {
  color: var(--muted);
  font-size: 13px;
  width: 70px;
  text-align: right;
  flex-shrink: 0;
}

.badge {
  font-size: 16px;
  flex-shrink: 0;
}

.loser-banner {
  text-align: center;
  margin-top: 16px;
  padding: 12px;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 8px;
  color: var(--danger);
  font-weight: 600;
  font-size: 15px;
}

.actions {
  display: flex;
  gap: 12px;
  margin-top: 20px;
}

.btn {
  flex: 1;
  padding: 12px;
  border: 1px solid var(--muted);
  border-radius: 8px;
  background: transparent;
  color: var(--text);
  font-size: 15px;
  cursor: pointer;
  transition: background 0.2s;
}

.btn:active {
  background: rgba(255, 255, 255, 0.05);
}

.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}

.btn.primary:active {
  background: #4f46e5;
}
</style>
