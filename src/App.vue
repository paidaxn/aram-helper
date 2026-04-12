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
const cameFrom = ref<Page>("status"); // 记录从哪个页面进入结果页
const connStatus = ref<"disconnected" | "connected">("disconnected");
const summonerName = ref("");
const gamePhase = ref("");
const errorMsg = ref("");
const loading = ref(false);

const gameResult = ref<GameResult | null>(null);
const matchList = ref<MatchSummary[]>([]);
const historyLoading = ref(false);

const loserTeam = computed(() => gameResult.value?.teams.find((t) => t.isLoser));

// ────── 轮询 ──────

let pollTimer: ReturnType<typeof setTimeout> | null = null;
function clearPoll() { if (pollTimer) { clearTimeout(pollTimer); pollTimer = null; } }

async function pollConnection() {
  try {
    summonerName.value = await invoke<string>("check_connection");
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
    // 选英雄阶段：抓取楼层顺序
    if (phase === "ChampSelect") {
      invoke("capture_champ_select").catch(() => {});
    }

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
  try {
    gameResult.value = await invoke<GameResult>("get_damage_ranking");
    cameFrom.value = "status";
    page.value = "result";
  } catch (e) {
    errorMsg.value = String(e);
    pollTimer = setTimeout(pollGamePhase, 3000);
  } finally { loading.value = false; }
}

async function viewLastGame() {
  loading.value = true;
  errorMsg.value = "";
  clearPoll();
  try {
    gameResult.value = await invoke<GameResult>("get_damage_ranking");
    cameFrom.value = "status";
    page.value = "result";
  } catch (e) { errorMsg.value = String(e); }
  finally { loading.value = false; }
}

async function loadHistory() {
  page.value = "history";
  historyLoading.value = true;
  try {
    matchList.value = await invoke<MatchSummary[]>("get_match_list", { count: 20 });
  } catch (e) { errorMsg.value = String(e); }
  finally { historyLoading.value = false; }
}

async function viewGame(gameId: number) {
  loading.value = true;
  errorMsg.value = "";
  try {
    gameResult.value = await invoke<GameResult>("get_game_result", { gameId });
    cameFrom.value = "history";
    page.value = "result";
  } catch (e) { errorMsg.value = String(e); }
  finally { loading.value = false; }
}

// 返回：根据来源决定去哪
function goBack() {
  if (page.value === "result" && cameFrom.value === "history") {
    page.value = "history";
  } else if (page.value === "history") {
    goHome();
  } else {
    goHome();
  }
}

function goHome() {
  page.value = "status";
  gameResult.value = null;
  errorMsg.value = "";
  connStatus.value === "connected" ? pollGamePhase() : pollConnection();
}

// ────── 工具 ──────

function fmt(n: number) { return n.toLocaleString(); }
function fmtDur(s: number) { return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, "0")}`; }
function fmtTime(ts: number) {
  const d = new Date(ts), now = new Date();
  const days = Math.floor((now.getTime() - d.getTime()) / 86400000);
  if (days === 0) return `今天 ${d.getHours()}:${d.getMinutes().toString().padStart(2, "0")}`;
  if (days === 1) return "昨天";
  if (days < 7) return `${days}天前`;
  return `${d.getMonth() + 1}/${d.getDate()}`;
}
function phaseText(p: string) {
  return ({ None:"大厅", Lobby:"房间中", Matchmaking:"匹配中", ReadyCheck:"确认中",
    ChampSelect:"选英雄中", InProgress:"游戏中", WaitingForStats:"结算中", EndOfGame:"对局结束" } as Record<string,string>)[p] || p;
}
function modeName(m: string) {
  return ({ ARAM:"大乱斗", KIWI:"大乱斗", CLASSIC:"匹配/排位", URF:"无限火力",
    CHERRY:"斗魂竞技场", NEXUSBLITZ:"极限闪击" } as Record<string,string>)[m] || m;
}

onMounted(() => pollConnection());
onUnmounted(() => clearPoll());
</script>

<template>
  <div class="app">
    <!-- ====== 状态页 ====== -->
    <template v-if="page === 'status'">
      <div v-if="connStatus === 'disconnected'" class="center-page">
        <div class="icon">🎮</div>
        <p class="title">ARAM 红包局助手</p>
        <p class="sub">等待客户端启动...</p>
        <div class="dots"><i /><i /><i /></div>
        <p v-if="errorMsg" class="err">{{ errorMsg }}</p>
      </div>

      <div v-else class="center-page">
        <div class="icon pulse">⚡</div>
        <p class="title">已连接</p>
        <p class="name">{{ summonerName }}</p>
        <p class="sub">{{ gamePhase ? phaseText(gamePhase) : "等待对局结束" }}</p>
        <div class="dots"><i /><i /><i /></div>
        <div class="row" style="margin-top:16px">
          <button class="btn-s" @click="viewLastGame" :disabled="loading">
            {{ loading ? "..." : "上一局" }}
          </button>
          <button class="btn-s" @click="loadHistory">历史</button>
        </div>
        <p v-if="errorMsg" class="err">{{ errorMsg }}</p>
      </div>
    </template>

    <!-- ====== 结果页 ====== -->
    <template v-else-if="page === 'result' && gameResult">
      <div class="page">
        <div class="nav">
          <button class="btn-back" @click="goBack">返回</button>
          <span class="nav-title">红包局结算</span>
          <span class="nav-meta">{{ modeName(gameResult.gameMode) }} {{ fmtDur(gameResult.gameDuration) }}</span>
        </div>

        <div v-if="!gameResult.isRedPacketGame" class="center-page" style="padding:24px 0">
          <p class="sub">本局好友不足 2 人，不算红包局</p>
        </div>

        <template v-else>
          <!-- 输家高亮 -->
          <div v-if="loserTeam" class="loser-box">
            <span class="loser-name">{{ loserTeam.players.map(p => p.summonerName).join("、") }}</span>
            <span class="loser-label">💸 发红包</span>
          </div>

          <!-- 各队 -->
          <div v-for="(team, i) in gameResult.teams" :key="i"
            class="team" :class="{ lose: team.isLoser }">
            <div class="team-top">
              <span class="team-name">{{ team.name }}</span>
              <span class="team-pts">{{ fmt(Math.round(team.score)) }} 分</span>
            </div>
            <div v-for="p in team.players" :key="p.summonerName" class="p-row">
              <span class="p-name">{{ p.summonerName }}</span>
              <span class="p-dmg">{{ fmt(p.damage) }}</span>
              <span class="p-tkn">{{ fmt(p.damageTaken) }}</span>
              <span class="p-kda">{{ p.kills }}/{{ p.deaths }}/{{ p.assists }}</span>
            </div>
          </div>
        </template>
      </div>
    </template>

    <!-- ====== 历史页 ====== -->
    <template v-else-if="page === 'history'">
      <div class="page scrollable">
        <div class="nav">
          <button class="btn-back" @click="goBack">返回</button>
          <span class="nav-title">历史对局</span>
        </div>

        <p v-if="historyLoading" class="sub" style="text-align:center;padding:24px">加载中...</p>

        <div v-else class="h-list">
          <div v-for="m in matchList" :key="m.gameId" class="h-item" @click="viewGame(m.gameId)">
            <span class="h-mode">{{ modeName(m.gameMode) }}</span>
            <span class="h-dur">{{ fmtDur(m.gameDuration) }}</span>
            <span class="h-time">{{ fmtTime(m.timestamp) }}</span>
            <span class="h-arrow">›</span>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style>
* { margin:0; padding:0; box-sizing:border-box; }
:root { --bg:#0f0f1a; --card:#1a1a2e; --text:#e0e0e0; --accent:#6366f1; --red:#ef4444; --muted:#666; }

body { background:var(--bg); color:var(--text); font-family:-apple-system,"PingFang SC","Microsoft YaHei",sans-serif; user-select:none; }

.app { height:100vh; display:flex; flex-direction:column; padding:14px; overflow:hidden; }

/* 居中页面 */
.center-page { flex:1; display:flex; flex-direction:column; align-items:center; justify-content:center; text-align:center; }
.icon { font-size:48px; margin-bottom:12px; }
.pulse { animation:pulse 2s ease-in-out infinite; }
@keyframes pulse { 0%,100%{opacity:1;transform:scale(1)} 50%{opacity:.7;transform:scale(1.08)} }
.title { font-size:20px; font-weight:700; margin-bottom:6px; }
.name { color:var(--accent); font-size:15px; font-weight:600; margin-bottom:4px; }
.sub { color:var(--muted); font-size:13px; }
.err { color:var(--red); font-size:11px; margin-top:8px; max-width:300px; word-break:break-all; }

.dots { display:flex; gap:4px; margin-top:14px; }
.dots i { width:5px; height:5px; border-radius:50%; background:var(--muted); animation:dot 1.4s ease-in-out infinite; display:block; }
.dots i:nth-child(2) { animation-delay:.2s; }
.dots i:nth-child(3) { animation-delay:.4s; }
@keyframes dot { 0%,80%,100%{opacity:.3;transform:scale(.8)} 40%{opacity:1;transform:scale(1.2)} }

.row { display:flex; gap:6px; }
.btn-s { padding:6px 14px; border:1px solid var(--muted); border-radius:6px; background:transparent; color:var(--text); font-size:12px; cursor:pointer; }
.btn-s:disabled { opacity:.4; }
.btn-s:active { background:rgba(255,255,255,.04); }

/* 页面容器 */
.page { flex:1; display:flex; flex-direction:column; overflow:hidden; }
.page.scrollable { overflow-y:auto; }

/* 导航 */
.nav { display:flex; align-items:center; gap:8px; margin-bottom:10px; }
.btn-back { padding:4px 10px; border:1px solid var(--muted); border-radius:4px; background:transparent; color:var(--text); font-size:12px; cursor:pointer; flex-shrink:0; }
.btn-back:active { background:rgba(255,255,255,.04); }
.nav-title { font-size:16px; font-weight:700; }
.nav-meta { color:var(--muted); font-size:11px; margin-left:auto; }

/* 输家高亮 */
.loser-box { text-align:center; padding:10px; margin-bottom:10px; background:rgba(239,68,68,.08); border-radius:8px; border:1px solid rgba(239,68,68,.2); }
.loser-name { color:var(--red); font-weight:700; font-size:15px; }
.loser-label { color:var(--red); font-size:13px; margin-left:6px; }

/* 队伍 */
.team { background:var(--card); border-radius:8px; padding:10px 12px; margin-bottom:5px; border:1.5px solid transparent; }
.team.lose { border-color:var(--red); background:rgba(239,68,68,.04); }
.team-top { display:flex; align-items:center; margin-bottom:4px; }
.team-name { font-weight:600; font-size:13px; flex:1; }
.team-pts { color:var(--accent); font-weight:700; font-size:13px; font-variant-numeric:tabular-nums; }

.p-row { display:flex; align-items:center; gap:4px; font-size:11px; padding:2px 0; }
.p-name { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.p-dmg { color:var(--text); width:48px; text-align:right; font-variant-numeric:tabular-nums; }
.p-tkn { color:#f59e0b; width:48px; text-align:right; font-variant-numeric:tabular-nums; }
.p-kda { color:var(--muted); width:44px; text-align:right; }

/* 历史列表 */
.h-list { flex:1; overflow-y:auto; }
.h-item { display:flex; align-items:center; gap:6px; padding:10px 12px; background:var(--card); border-radius:6px; margin-bottom:3px; cursor:pointer; font-size:12px; }
.h-item:active { background:rgba(99,102,241,.08); }
.h-mode { font-weight:600; flex:1; }
.h-dur { color:var(--muted); }
.h-time { color:var(--muted); width:52px; text-align:right; }
.h-arrow { color:var(--muted); font-size:16px; }
</style>
