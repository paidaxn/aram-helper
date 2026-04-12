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
const loadingLastGame = ref(false);

// 数据类型
interface Player {
  summonerName: string;
  championId: number;
  damage: number;
  damageTaken: number;
  score: number;
  kills: number;
  deaths: number;
  assists: number;
  floor: number;
  isFriend: boolean;
}

interface Team {
  name: string;
  players: Player[];
  score: number;
  isLoser: boolean;
}

interface GameResult {
  allPlayers: Player[];
  friendCount: number;
  teams: Team[];
  isRedPacketGame: boolean;
}

const gameResult = ref<GameResult | null>(null);

// 输的队伍
const loserTeam = computed(() => gameResult.value?.teams.find((t) => t.isLoser));

// 轮询定时器
let pollTimer: ReturnType<typeof setTimeout> | null = null;

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
      await fetchResult();
    } else {
      pollTimer = setTimeout(pollGamePhase, 3000);
    }
  } catch {
    status.value = "disconnected";
    pollTimer = setTimeout(pollConnection, 5000);
  }
}

// 获取对局结果
async function fetchResult() {
  try {
    const result = await invoke<GameResult>("get_damage_ranking");
    gameResult.value = result;
    status.value = "result";
  } catch (e) {
    errorMsg.value = String(e);
    pollTimer = setTimeout(pollGamePhase, 3000);
  }
}

// 手动查看上一局
async function viewLastGame() {
  loadingLastGame.value = true;
  errorMsg.value = "";
  clearPoll();
  try {
    const result = await invoke<GameResult>("get_damage_ranking");
    gameResult.value = result;
    status.value = "result";
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loadingLastGame.value = false;
  }
}

// 复制结果到剪贴板
async function copyResult() {
  if (!gameResult.value) return;
  const r = gameResult.value;

  let text = "🏆 ARAM 红包局结算\n\n";

  if (r.isRedPacketGame) {
    for (const team of r.teams) {
      const mark = team.isLoser ? " 💸" : "";
      text += `【${team.name}】分数：${Math.round(team.score).toLocaleString()}${mark}\n`;
      for (const p of team.players) {
        text += `  ${p.summonerName} | 输出 ${p.damage.toLocaleString()} | 承伤 ${p.damageTaken.toLocaleString()} | ${p.kills}/${p.deaths}/${p.assists}\n`;
      }
    }
    if (loserTeam.value) {
      const names = loserTeam.value.players.map((p) => p.summonerName).join("、");
      text += `\n${names} 发红包！`;
    }
  } else {
    text += "本局无好友参与，不算红包局";
  }

  await navigator.clipboard.writeText(text);
  copied.value = true;
  setTimeout(() => (copied.value = false), 2000);
}

// 继续等待下一局
function waitNext() {
  status.value = "connected";
  gamePhase.value = "";
  gameResult.value = null;
  errorMsg.value = "";
  pollGamePhase();
}

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
    <div v-else-if="gameResult" class="result-page">
      <h2>🏆 红包局结算</h2>
      <p class="mode-tag">{{ gameResult.friendCount }} 人红包局</p>

      <!-- 不是红包局 -->
      <div v-if="!gameResult.isRedPacketGame" class="no-game">
        <p>本局好友不足 2 人，不算红包局</p>
        <p class="hint">需要至少 2 个好友一起玩才能触发红包局</p>
      </div>

      <!-- 红包局结果 -->
      <template v-else>
        <!-- 各队伍 -->
        <div
          v-for="(team, ti) in gameResult.teams"
          :key="ti"
          class="team-card"
          :class="{ loser: team.isLoser }"
        >
          <div class="team-header">
            <span class="team-name">{{ team.name }}</span>
            <span class="team-score">{{ Math.round(team.score).toLocaleString() }} 分</span>
            <span v-if="team.isLoser" class="team-badge">💸 发红包</span>
          </div>
          <div v-for="p in team.players" :key="p.summonerName" class="player-row">
            <span class="player-name">{{ p.summonerName }}</span>
            <span class="player-stat">输出 {{ p.damage.toLocaleString() }}</span>
            <span class="player-stat">承伤 {{ p.damageTaken.toLocaleString() }}</span>
            <span class="player-kda">{{ p.kills }}/{{ p.deaths }}/{{ p.assists }}</span>
          </div>
        </div>

        <!-- 惩罚结果 -->
        <div v-if="loserTeam" class="loser-banner">
          {{ loserTeam.players.map((p) => p.summonerName).join("、") }} 发红包！
        </div>

        <!-- 所有队员数据 -->
        <details class="all-players-toggle">
          <summary>查看全队 {{ gameResult.allPlayers.length }} 人数据</summary>
          <div class="all-players">
            <div
              v-for="p in gameResult.allPlayers"
              :key="p.floor"
              class="mini-row"
              :class="{ friend: p.isFriend }"
            >
              <span class="mini-floor">{{ p.floor }}楼</span>
              <span class="mini-name">{{ p.summonerName }}</span>
              <span class="mini-score">{{ Math.round(p.score).toLocaleString() }}分</span>
              <span v-if="p.isFriend" class="mini-tag">好友</span>
            </div>
          </div>
        </details>
      </template>

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
  --success: #22c55e;
}

* { margin: 0; padding: 0; box-sizing: border-box; }

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

.status-page { text-align: center; }

.logo { font-size: 64px; margin-bottom: 24px; }

.pulse {
  animation: pulse 2s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.7; transform: scale(1.1); }
}

h1 { font-size: 28px; font-weight: 700; margin-bottom: 12px; }

.summoner {
  color: var(--accent);
  font-size: 18px;
  font-weight: 600;
  margin-bottom: 8px;
}

.hint { color: var(--muted); font-size: 16px; }

.error {
  color: var(--danger);
  margin-top: 12px;
  font-size: 14px;
  max-width: 360px;
  word-break: break-all;
}

.dots {
  display: flex;
  justify-content: center;
  gap: 6px;
  margin-top: 24px;
}

.dots span {
  width: 8px; height: 8px; border-radius: 50%;
  background: var(--muted);
  animation: dot 1.4s ease-in-out infinite;
}
.dots span:nth-child(2) { animation-delay: 0.2s; }
.dots span:nth-child(3) { animation-delay: 0.4s; }

@keyframes dot {
  0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
  40% { opacity: 1; transform: scale(1.2); }
}

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
.btn-last-game:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-last-game:not(:disabled):active { background: rgba(255,255,255,0.05); }

/* 结果页 */
.result-page { width: 100%; max-width: 460px; }

h2 { font-size: 22px; text-align: center; margin-bottom: 4px; }

.mode-tag {
  text-align: center;
  color: var(--accent);
  font-size: 14px;
  margin-bottom: 16px;
}

.no-game {
  text-align: center;
  padding: 32px 0;
}
.no-game p:first-child {
  font-size: 16px;
  margin-bottom: 8px;
}

/* 队伍卡片 */
.team-card {
  background: var(--card);
  border-radius: 12px;
  padding: 14px 16px;
  margin-bottom: 8px;
  border: 2px solid transparent;
}

.team-card.loser {
  border-color: var(--danger);
  background: rgba(239, 68, 68, 0.08);
}

.team-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.team-name {
  font-weight: 700;
  font-size: 15px;
}

.team-score {
  margin-left: auto;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--accent);
}

.team-badge {
  font-size: 13px;
  color: var(--danger);
  font-weight: 600;
}

.player-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  padding: 4px 0;
  color: var(--text);
}

.player-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.player-stat {
  color: var(--muted);
  font-size: 12px;
  flex-shrink: 0;
}

.player-kda {
  color: var(--muted);
  font-size: 12px;
  width: 60px;
  text-align: right;
  flex-shrink: 0;
}

.loser-banner {
  text-align: center;
  margin-top: 12px;
  padding: 12px;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 8px;
  color: var(--danger);
  font-weight: 600;
  font-size: 16px;
}

/* 全队数据折叠 */
.all-players-toggle {
  margin-top: 12px;
  font-size: 13px;
  color: var(--muted);
}

.all-players-toggle summary {
  cursor: pointer;
  padding: 8px 0;
}

.all-players {
  margin-top: 4px;
}

.mini-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  font-size: 13px;
  border-radius: 4px;
}

.mini-row.friend {
  background: rgba(99, 102, 241, 0.08);
}

.mini-floor {
  width: 30px;
  color: var(--muted);
  flex-shrink: 0;
}

.mini-name { flex: 1; }

.mini-score {
  font-variant-numeric: tabular-nums;
  flex-shrink: 0;
}

.mini-tag {
  font-size: 11px;
  color: var(--accent);
  border: 1px solid var(--accent);
  border-radius: 4px;
  padding: 1px 4px;
  flex-shrink: 0;
}

.actions {
  display: flex;
  gap: 12px;
  margin-top: 16px;
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
}
.btn:active { background: rgba(255,255,255,0.05); }

.btn.primary {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
  font-weight: 600;
}
.btn.primary:active { background: #4f46e5; }
</style>
