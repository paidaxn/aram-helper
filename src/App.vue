<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

// ────── 类型 ──────

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
  gameId: number;
  gameMode: string;
  gameDuration: number;
  timestamp: number;
  allPlayers: Player[];
  friendCount: number;
  teams: Team[];
  isRedPacketGame: boolean;
}

interface MatchSummary {
  gameId: number;
  gameMode: string;
  gameDuration: number;
  timestamp: number;
  championId: number;
}

// ────── 状态 ──────

type Page = "status" | "result" | "history";
const page = ref<Page>("status");

type ConnStatus = "disconnected" | "connected";
const connStatus = ref<ConnStatus>("disconnected");
const summonerName = ref("");
const gamePhase = ref("");
const errorMsg = ref("");
const copied = ref(false);
const loading = ref(false);

// 当前查看的对局结果
const gameResult = ref<GameResult | null>(null);

// 历史对局列表
const matchList = ref<MatchSummary[]>([]);
const historyLoading = ref(false);

// 输的队伍
const loserTeam = computed(() => gameResult.value?.teams.find((t) => t.isLoser));

// ────── 轮询 ──────

let pollTimer: ReturnType<typeof setTimeout> | null = null;

function clearPoll() {
  if (pollTimer) {
    clearTimeout(pollTimer);
    pollTimer = null;
  }
}

async function pollConnection() {
  try {
    const name = await invoke<string>("check_connection");
    summonerName.value = name;
    errorMsg.value = "";
    connStatus.value = "connected";
    pollGamePhase();
  } catch (e) {
    connStatus.value = "disconnected";
    errorMsg.value = String(e);
    pollTimer = setTimeout(pollConnection, 5000);
  }
}

async function pollGamePhase() {
  try {
    const phase = await invoke<string>("get_gameflow_phase");
    gamePhase.value = phase;

    if (phase === "EndOfGame" || phase === "WaitingForStats") {
      await fetchLatestResult();
    } else {
      pollTimer = setTimeout(pollGamePhase, 3000);
    }
  } catch {
    connStatus.value = "disconnected";
    pollTimer = setTimeout(pollConnection, 5000);
  }
}

// ────── 数据获取 ──────

async function fetchLatestResult() {
  loading.value = true;
  errorMsg.value = "";
  try {
    const result = await invoke<GameResult>("get_damage_ranking");
    gameResult.value = result;
    page.value = "result";
  } catch (e) {
    errorMsg.value = String(e);
    pollTimer = setTimeout(pollGamePhase, 3000);
  } finally {
    loading.value = false;
  }
}

async function viewLastGame() {
  loading.value = true;
  errorMsg.value = "";
  clearPoll();
  try {
    const result = await invoke<GameResult>("get_damage_ranking");
    gameResult.value = result;
    page.value = "result";
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadHistory() {
  page.value = "history";
  historyLoading.value = true;
  try {
    matchList.value = await invoke<MatchSummary[]>("get_match_list", { count: 20 });
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    historyLoading.value = false;
  }
}

async function viewGame(gameId: number) {
  loading.value = true;
  errorMsg.value = "";
  try {
    const result = await invoke<GameResult>("get_game_result", { gameId });
    gameResult.value = result;
    page.value = "result";
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

function goHome() {
  page.value = "status";
  gameResult.value = null;
  errorMsg.value = "";
  if (connStatus.value === "connected") {
    pollGamePhase();
  } else {
    pollConnection();
  }
}

// ────── 工具 ──────

async function copyResult() {
  if (!gameResult.value) return;
  const r = gameResult.value;
  let text = "🏆 ARAM 红包局结算\n\n";
  if (r.isRedPacketGame) {
    for (const team of r.teams) {
      const mark = team.isLoser ? " 💸" : "";
      text += `【${team.name}】${Math.round(team.score).toLocaleString()} 分${mark}\n`;
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

function formatDuration(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatTime(ts: number): string {
  const d = new Date(ts);
  const now = new Date();
  const diff = now.getTime() - d.getTime();
  const days = Math.floor(diff / 86400000);
  if (days === 0) return `今天 ${d.getHours()}:${d.getMinutes().toString().padStart(2, "0")}`;
  if (days === 1) return "昨天";
  if (days < 7) return `${days}天前`;
  return `${d.getMonth() + 1}/${d.getDate()}`;
}

function phaseText(p: string): string {
  const m: Record<string, string> = {
    None: "大厅", Lobby: "房间中", Matchmaking: "匹配中", ReadyCheck: "确认中",
    ChampSelect: "选英雄中", GameStart: "启动中", InProgress: "游戏中",
    WaitingForStats: "等待结算", EndOfGame: "对局结束",
  };
  return m[p] || p;
}

function modeName(mode: string): string {
  const m: Record<string, string> = {
    ARAM: "大乱斗", CLASSIC: "匹配/排位", URF: "无限火力",
    NEXUSBLITZ: "极限闪击", CHERRY: "斗魂竞技场",
  };
  return m[mode] || mode;
}

onMounted(() => pollConnection());
onUnmounted(() => clearPoll());
</script>

<template>
  <div class="app">
    <!-- ====== 状态页 ====== -->
    <template v-if="page === 'status'">
      <!-- 未连接 -->
      <div v-if="connStatus === 'disconnected'" class="status-page">
        <div class="logo">🎮</div>
        <h1>ARAM 红包局助手</h1>
        <p class="hint">等待英雄联盟客户端启动...</p>
        <div class="dots"><span /><span /><span /></div>
        <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
      </div>

      <!-- 已连接 -->
      <div v-else class="status-page">
        <div class="logo pulse">⚡</div>
        <h1>已连接</h1>
        <p class="summoner">{{ summonerName }}</p>
        <p class="hint">
          {{ gamePhase ? `当前：${phaseText(gamePhase)}` : "等待游戏结束后自动显示" }}
        </p>
        <div class="dots"><span /><span /><span /></div>
        <div class="status-actions">
          <button class="btn-sm" @click="viewLastGame" :disabled="loading">
            {{ loading ? "加载中..." : "查看上一局" }}
          </button>
          <button class="btn-sm" @click="loadHistory">历史记录</button>
        </div>
        <p v-if="errorMsg" class="error">{{ errorMsg }}</p>
      </div>
    </template>

    <!-- ====== 结果页 ====== -->
    <template v-else-if="page === 'result' && gameResult">
      <div class="result-page">
        <div class="page-header">
          <button class="btn-back" @click="goHome">← 返回</button>
          <span class="page-title">红包局结算</span>
          <span class="page-meta">{{ modeName(gameResult.gameMode) }} · {{ formatDuration(gameResult.gameDuration) }}</span>
        </div>

        <!-- 非红包局 -->
        <div v-if="!gameResult.isRedPacketGame" class="no-game">
          <p>本局好友不足 2 人</p>
          <p class="hint">不算红包局</p>
        </div>

        <!-- 红包局结果 -->
        <template v-else>
          <p class="mode-tag">{{ gameResult.friendCount }} 人红包局</p>

          <div
            v-for="(team, ti) in gameResult.teams"
            :key="ti"
            class="team-card"
            :class="{ loser: team.isLoser }"
          >
            <div class="team-header">
              <span class="team-name">{{ team.name }}</span>
              <span class="team-score">{{ Math.round(team.score).toLocaleString() }}</span>
              <span v-if="team.isLoser" class="team-badge">💸</span>
            </div>
            <div v-for="p in team.players" :key="p.summonerName" class="player-row">
              <span class="player-name">{{ p.summonerName }}</span>
              <span class="player-stat">{{ p.damage.toLocaleString() }}</span>
              <span class="player-stat taken">{{ p.damageTaken.toLocaleString() }}</span>
              <span class="player-kda">{{ p.kills }}/{{ p.deaths }}/{{ p.assists }}</span>
            </div>
          </div>

          <!-- 惩罚结果 -->
          <div v-if="loserTeam" class="loser-banner">
            <span class="loser-names">{{ loserTeam.players.map((p) => p.summonerName).join("、") }}</span>
            <span>发红包！</span>
          </div>

          <!-- 全队数据 -->
          <details class="details-toggle">
            <summary>全队 {{ gameResult.allPlayers.length }} 人数据</summary>
            <div class="mini-list">
              <div v-for="p in gameResult.allPlayers" :key="p.floor" class="mini-row" :class="{ friend: p.isFriend }">
                <span class="mini-name">{{ p.summonerName }}</span>
                <span class="mini-score">{{ Math.round(p.score).toLocaleString() }}分</span>
                <span v-if="p.isFriend" class="mini-tag">好友</span>
              </div>
            </div>
          </details>
        </template>

        <div class="actions">
          <button class="btn primary" @click="copyResult">{{ copied ? "已复制 ✓" : "复制结果" }}</button>
          <button class="btn" @click="loadHistory">历史</button>
        </div>
      </div>
    </template>

    <!-- ====== 历史页 ====== -->
    <template v-else-if="page === 'history'">
      <div class="history-page">
        <div class="page-header">
          <button class="btn-back" @click="goHome">← 返回</button>
          <span class="page-title">历史对局</span>
        </div>

        <div v-if="historyLoading" class="loading-hint">加载中...</div>

        <div v-else-if="matchList.length === 0" class="loading-hint">暂无对局记录</div>

        <div v-else class="match-list">
          <div
            v-for="m in matchList"
            :key="m.gameId"
            class="match-item"
            @click="viewGame(m.gameId)"
          >
            <span class="match-mode">{{ modeName(m.gameMode) }}</span>
            <span class="match-duration">{{ formatDuration(m.gameDuration) }}</span>
            <span class="match-time">{{ formatTime(m.timestamp) }}</span>
            <span class="match-arrow">›</span>
          </div>
        </div>
      </div>
    </template>
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
  padding: 16px;
}

/* 状态页 */
.status-page {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
}

.logo { font-size: 56px; margin-bottom: 20px; }
.pulse { animation: pulse 2s ease-in-out infinite; }
@keyframes pulse {
  0%,100% { opacity:1; transform:scale(1); }
  50% { opacity:0.7; transform:scale(1.1); }
}

h1 { font-size: 24px; font-weight: 700; margin-bottom: 10px; }

.summoner { color: var(--accent); font-size: 16px; font-weight: 600; margin-bottom: 6px; }
.hint { color: var(--muted); font-size: 14px; }
.error { color: var(--danger); margin-top: 10px; font-size: 12px; max-width: 320px; word-break: break-all; }

.dots { display: flex; justify-content: center; gap: 5px; margin-top: 20px; }
.dots span { width: 6px; height: 6px; border-radius: 50%; background: var(--muted); animation: dot 1.4s ease-in-out infinite; }
.dots span:nth-child(2) { animation-delay: 0.2s; }
.dots span:nth-child(3) { animation-delay: 0.4s; }
@keyframes dot {
  0%,80%,100% { opacity:0.3; transform:scale(0.8); }
  40% { opacity:1; transform:scale(1.2); }
}

.status-actions { display: flex; gap: 8px; margin-top: 20px; }

.btn-sm {
  padding: 8px 16px;
  border: 1px solid var(--muted);
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  cursor: pointer;
}
.btn-sm:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-sm:not(:disabled):active { background: rgba(255,255,255,0.05); }

/* 页面头部 */
.page-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 12px;
  flex-wrap: wrap;
}

.btn-back {
  background: none;
  border: none;
  color: var(--accent);
  font-size: 14px;
  cursor: pointer;
  padding: 4px 0;
}

.page-title { font-size: 18px; font-weight: 700; }
.page-meta { color: var(--muted); font-size: 12px; margin-left: auto; }

/* 结果页 */
.result-page { flex: 1; display: flex; flex-direction: column; }

.mode-tag { color: var(--accent); font-size: 13px; margin-bottom: 10px; }

.no-game { text-align: center; padding: 32px 0; }
.no-game p:first-child { font-size: 15px; margin-bottom: 6px; }

.team-card {
  background: var(--card);
  border-radius: 10px;
  padding: 12px 14px;
  margin-bottom: 6px;
  border: 2px solid transparent;
}

.team-card.loser {
  border-color: var(--danger);
  background: rgba(239,68,68,0.06);
}

.team-header {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 6px;
}

.team-name { font-weight: 600; font-size: 14px; flex: 1; }
.team-score { font-weight: 700; font-variant-numeric: tabular-nums; color: var(--accent); font-size: 14px; }
.team-badge { font-size: 14px; }

.player-row {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  padding: 3px 0;
}

.player-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.player-stat { color: var(--muted); font-size: 11px; min-width: 50px; text-align: right; }
.player-stat.taken { color: #f59e0b; }
.player-kda { color: var(--muted); font-size: 11px; width: 50px; text-align: right; }

.loser-banner {
  text-align: center;
  margin: 10px 0;
  padding: 10px;
  background: rgba(239,68,68,0.08);
  border-radius: 8px;
  color: var(--danger);
  font-size: 15px;
}

.loser-names { font-weight: 700; }

.details-toggle { font-size: 12px; color: var(--muted); margin-top: 8px; }
.details-toggle summary { cursor: pointer; padding: 6px 0; }

.mini-list { margin-top: 4px; }
.mini-row {
  display: flex; align-items: center; gap: 6px;
  padding: 3px 6px; font-size: 12px; border-radius: 4px;
}
.mini-row.friend { background: rgba(99,102,241,0.06); }
.mini-name { flex: 1; }
.mini-score { font-variant-numeric: tabular-nums; }
.mini-tag { font-size: 10px; color: var(--accent); border: 1px solid var(--accent); border-radius: 3px; padding: 0 3px; }

/* 历史页 */
.history-page { flex: 1; display: flex; flex-direction: column; }

.loading-hint { text-align: center; color: var(--muted); padding: 32px 0; font-size: 14px; }

.match-list { flex: 1; overflow-y: auto; }

.match-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 14px;
  background: var(--card);
  border-radius: 8px;
  margin-bottom: 4px;
  cursor: pointer;
  font-size: 13px;
}
.match-item:active { background: rgba(99,102,241,0.1); }

.match-mode { font-weight: 600; flex: 1; }
.match-duration { color: var(--muted); font-size: 12px; }
.match-time { color: var(--muted); font-size: 12px; width: 60px; text-align: right; }
.match-arrow { color: var(--muted); font-size: 18px; }

/* 按钮 */
.actions { display: flex; gap: 8px; margin-top: auto; padding-top: 12px; }

.btn {
  flex: 1; padding: 10px; border: 1px solid var(--muted); border-radius: 8px;
  background: transparent; color: var(--text); font-size: 14px; cursor: pointer;
}
.btn:active { background: rgba(255,255,255,0.05); }

.btn.primary { background: var(--accent); border-color: var(--accent); color: #fff; font-weight: 600; }
.btn.primary:active { background: #4f46e5; }
</style>
