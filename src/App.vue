<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { RULES, getRule, type GameResult } from "./rules";

// ────── 类型 ──────

interface MatchSummary {
  gameId: number;
  gameMode: string;
  gameDuration: number;
  timestamp: number;
  championId: number;
}

// ────── 状态 ──────

type Page = "status" | "result" | "history" | "settings";
const page = ref<Page>("status");
const cameFrom = ref<Page>("status");
const connStatus = ref<"disconnected" | "connected">("disconnected");
const summonerName = ref("");
const gamePhase = ref("");
const errorMsg = ref("");
const loading = ref(false);

const gameResult = ref<GameResult | null>(null);
const matchList = ref<MatchSummary[]>([]);
const historyLoading = ref(false);

// 当前规则（从 localStorage 读取）
const currentRuleId = ref<string>(localStorage.getItem("ruleId") || "classic");
watch(currentRuleId, (v) => localStorage.setItem("ruleId", v));

const currentRule = computed(() => getRule(currentRuleId.value));
const ruleResult = computed(() => {
  if (!gameResult.value) return null;
  return currentRule.value.calculate(gameResult.value);
});
const loserTeam = computed(() => ruleResult.value?.teams.find((t) => t.isLoser));

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

    if (phase === "ChampSelect") {
      await invoke("capture_champ_select").catch(() => {});
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
    startAutoDismissPoll();
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
    gameResult.value = await invoke<GameResult>("get_damage_ranking");
    cameFrom.value = "status";
    page.value = "result";
    startAutoDismissPoll();
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

// 结果页继续轮询游戏状态，新对局开始时自动返回
async function startAutoDismissPoll() {
  clearPoll();
  const tick = async () => {
    try {
      const phase = await invoke<string>("get_gameflow_phase");
      gamePhase.value = phase;
      // 新对局开始 → 自动回到首页
      const newGamePhases = ["ChampSelect", "ReadyCheck", "Matchmaking", "GameStart", "InProgress"];
      if (newGamePhases.includes(phase)) {
        goHome();
        return;
      }
      pollTimer = setTimeout(tick, 3000);
    } catch {
      connStatus.value = "disconnected";
      pollTimer = setTimeout(pollConnection, 5000);
    }
  };
  pollTimer = setTimeout(tick, 3000);
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
    gameResult.value = await invoke<GameResult>("get_game_result", { gameId });
    cameFrom.value = "history";
    page.value = "result";
    startAutoDismissPoll();
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

function goBack() {
  if (page.value === "result" && cameFrom.value === "history") {
    page.value = "history";
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

function openSettings() {
  page.value = "settings";
}

// ────── 工具 ──────

function fmt(n: number) { return Math.round(n).toLocaleString(); }
function fmtDur(s: number) { return `${Math.floor(s / 60)}:${(s % 60).toString().padStart(2, "0")}`; }
function fmtScore(s: number, label: string) {
  if (label === "OP 分") return s.toFixed(1);
  return fmt(s);
}
function fmtTime(ts: number) {
  const d = new Date(ts), now = new Date();
  const days = Math.floor((now.getTime() - d.getTime()) / 86400000);
  const hh = d.getHours().toString().padStart(2, "0");
  const mm = d.getMinutes().toString().padStart(2, "0");
  if (days === 0) return `今天 ${hh}:${mm}`;
  if (days === 1) return `昨天 ${hh}:${mm}`;
  if (days < 7) return `${days}天前`;
  return `${d.getMonth() + 1}/${d.getDate()}`;
}
function phaseText(p: string) {
  return ({ None:"大厅", Lobby:"房间中", Matchmaking:"匹配中", ReadyCheck:"确认中",
    ChampSelect:"选英雄中", InProgress:"游戏中", WaitingForStats:"结算中", EndOfGame:"对局结束" } as Record<string,string>)[p] || p;
}
function modeName(m: string) {
  return ({ ARAM:"大乱斗", KIWI:"大乱斗", CLASSIC:"匹配/排位", URF:"无限火力", CHERRY:"斗魂竞技场" } as Record<string,string>)[m] || m;
}

onMounted(() => pollConnection());
onUnmounted(() => clearPoll());
</script>

<template>
  <div class="app">
    <!-- ====== 状态页 ====== -->
    <template v-if="page === 'status'">
      <div v-if="connStatus === 'disconnected'" class="center-page">
        <div class="brand-logo">
          <svg viewBox="0 0 64 64" width="56" height="56">
            <defs>
              <linearGradient id="g1" x1="0" y1="0" x2="1" y2="1">
                <stop offset="0%" stop-color="#A78BFA"/>
                <stop offset="100%" stop-color="#7C3AED"/>
              </linearGradient>
            </defs>
            <circle cx="32" cy="32" r="28" fill="none" stroke="url(#g1)" stroke-width="2" opacity="0.4"/>
            <rect x="18" y="22" width="28" height="24" rx="3" fill="#F59E0B"/>
            <rect x="18" y="22" width="28" height="7" rx="3" fill="#EA580C"/>
            <circle cx="32" cy="36" r="5" fill="#FBBF24"/>
          </svg>
        </div>
        <p class="title">LOL 红包局助手</p>
        <p class="sub">等待客户端启动</p>
        <div class="dots"><i /><i /><i /></div>
        <button class="btn-s" style="margin-top:16px" @click="openSettings">设置</button>
        <p v-if="errorMsg" class="err">{{ errorMsg }}</p>
      </div>

      <div v-else class="center-page">
        <div class="status-ring">
          <svg viewBox="0 0 64 64" width="48" height="48">
            <circle cx="32" cy="32" r="28" fill="none" stroke="#7C3AED" stroke-width="2" opacity="0.3"/>
            <path d="M32 14 L32 30 L44 30 L28 50 L28 34 L16 34 Z" fill="#A78BFA"/>
          </svg>
        </div>
        <p class="title">已连接</p>
        <p class="name">{{ summonerName }}</p>
        <p class="sub">{{ gamePhase ? phaseText(gamePhase) : "等待对局结束" }}</p>
        <p class="rule-tag">规则：{{ currentRule.name }}</p>
        <div class="dots"><i /><i /><i /></div>
        <div class="row" style="margin-top:20px">
          <button class="btn-s" @click="viewLastGame" :disabled="loading">
            {{ loading ? "加载中" : "上一局" }}
          </button>
          <button class="btn-s" @click="loadHistory">历史</button>
          <button class="btn-s" @click="openSettings">设置</button>
        </div>
        <p v-if="errorMsg" class="err">{{ errorMsg }}</p>
      </div>
    </template>

    <!-- ====== 结果页 ====== -->
    <template v-else-if="page === 'result' && gameResult && ruleResult">
      <div class="page">
        <div class="nav">
          <button class="btn-back" @click="goBack">返回</button>
          <span class="nav-title">{{ currentRule.name }}</span>
          <span class="nav-meta">{{ modeName(gameResult.gameMode) }} {{ fmtDur(gameResult.gameDuration) }}</span>
        </div>

        <div v-if="!ruleResult.isRedPacketGame" class="center-page" style="padding:24px 0">
          <p class="sub">本局好友不足 2 人，不算红包局</p>
        </div>

        <template v-else>
          <p v-if="ruleResult.needsFloors && !gameResult.hasAccurateFloors" class="approx-tip">
            ⚠ 历史对局，分队可能不准
          </p>

          <div v-if="loserTeam" class="loser-box">
            <span class="loser-name">{{ loserTeam.players.map(p => p.summonerName).join("、") }}</span>
            <span class="loser-label">💸 发红包</span>
          </div>

          <div
            v-for="(team, i) in ruleResult.teams"
            :key="i"
            class="team"
            :class="{ lose: team.isLoser }"
          >
            <div class="team-top">
              <span class="team-name">{{ team.name }}</span>
              <span class="team-pts">{{ fmtScore(team.score, ruleResult.scoreLabel) }} {{ ruleResult.scoreLabel }}</span>
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

    <!-- ====== 设置页 ====== -->
    <template v-else-if="page === 'settings'">
      <div class="page scrollable">
        <div class="nav">
          <button class="btn-back" @click="goBack">返回</button>
          <span class="nav-title">设置</span>
        </div>

        <p class="section-label">算分规则</p>
        <div class="rule-list">
          <label
            v-for="r in RULES"
            :key="r.id"
            class="rule-item"
            :class="{ selected: currentRuleId === r.id }"
          >
            <input type="radio" :value="r.id" v-model="currentRuleId" />
            <div class="rule-body">
              <div class="rule-name">{{ r.name }}</div>
              <div class="rule-desc">{{ r.description }}</div>
            </div>
          </label>
        </div>

        <p class="section-label" style="margin-top:16px">关于</p>
        <div class="about-box">
          <p class="about-line">LOL 红包局助手 v0.1.0</p>
          <p class="about-line muted">连接本地 LCU API，自动结算对局</p>
        </div>
      </div>
    </template>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Chakra+Petch:wght@500;600;700&family=Inter:wght@400;500;600;700&display=swap');

* { margin:0; padding:0; box-sizing:border-box; }

:root {
  --bg:            #0A0E1A;
  --bg-gradient:   radial-gradient(ellipse at top right, rgba(124,58,237,0.12) 0%, transparent 50%),
                   radial-gradient(ellipse at bottom left, rgba(245,158,11,0.06) 0%, transparent 50%);
  --surface:       #151B2E;
  --surface-2:     #1E2540;
  --border:        #2A3451;
  --border-light:  #3A4563;

  --text:          #F1F5F9;
  --text-2:        #CBD5E1;
  --muted:         #64748B;

  --primary:       #7C3AED;
  --primary-hi:    #A78BFA;
  --primary-glow:  rgba(124, 58, 237, 0.35);
  --gold:          #F59E0B;
  --gold-hi:       #FBBF24;
  --danger:        #EF4444;
  --danger-glow:   rgba(239, 68, 68, 0.25);

  --font-display:  'Chakra Petch', 'Segoe UI', sans-serif;
  --font-body:     'Inter', 'Segoe UI Variable', 'Segoe UI', -apple-system, 'PingFang SC', 'Microsoft YaHei', sans-serif;
}

html, body {
  background: var(--bg);
  color: var(--text);
  font-family: var(--font-body);
  font-size: 13px;
  user-select: none;
  -webkit-font-smoothing: antialiased;
}

body::before {
  content: '';
  position: fixed;
  inset: 0;
  background: var(--bg-gradient);
  pointer-events: none;
  z-index: 0;
}

.app {
  position: relative;
  height: 100vh;
  display: flex;
  flex-direction: column;
  padding: 16px;
  overflow: hidden;
  z-index: 1;
}

/* ────── 居中页面 ────── */
.center-page {
  flex: 1; display: flex; flex-direction: column;
  align-items: center; justify-content: center; text-align: center;
}

.brand-logo, .status-ring {
  margin-bottom: 16px;
  filter: drop-shadow(0 0 20px var(--primary-glow));
}
.status-ring { animation: breathe 2.4s ease-in-out infinite; }
@keyframes breathe {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.85; transform: scale(1.05); }
}

.title {
  font-family: var(--font-display);
  font-size: 22px; font-weight: 700; letter-spacing: 0.5px;
  margin-bottom: 8px;
}

.name {
  font-family: var(--font-display);
  color: var(--primary-hi);
  font-size: 16px; font-weight: 600; margin-bottom: 6px;
  letter-spacing: 0.3px;
  text-shadow: 0 0 12px var(--primary-glow);
}

.sub { color: var(--muted); font-size: 13px; }

.rule-tag {
  margin-top: 8px;
  color: var(--gold);
  font-size: 11px;
  padding: 3px 10px;
  background: rgba(245, 158, 11, 0.08);
  border: 1px solid rgba(245, 158, 11, 0.25);
  border-radius: 12px;
}

.err {
  color: var(--danger); font-size: 11px; margin-top: 10px;
  max-width: 320px; word-break: break-all;
  padding: 6px 10px; background: rgba(239, 68, 68, 0.08);
  border-radius: 6px; border: 1px solid rgba(239, 68, 68, 0.2);
}

.dots { display: flex; gap: 5px; margin-top: 18px; }
.dots i {
  width: 6px; height: 6px; border-radius: 50%;
  background: var(--primary);
  animation: dot 1.4s ease-in-out infinite;
  display: block;
  box-shadow: 0 0 8px var(--primary-glow);
}
.dots i:nth-child(2) { animation-delay: .2s; }
.dots i:nth-child(3) { animation-delay: .4s; }
@keyframes dot {
  0%,80%,100% { opacity: .3; transform: scale(.8); }
  40% { opacity: 1; transform: scale(1.2); }
}

.row { display: flex; gap: 8px; flex-wrap: wrap; justify-content: center; }

.btn-s {
  padding: 8px 18px;
  border: 1px solid var(--border-light);
  border-radius: 6px;
  background: var(--surface);
  color: var(--text-2);
  font-family: var(--font-display);
  font-size: 12px; font-weight: 600; letter-spacing: 0.5px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.btn-s:hover:not(:disabled) {
  border-color: var(--primary); color: var(--text); background: var(--surface-2);
}
.btn-s:disabled { opacity: .4; cursor: not-allowed; }
.btn-s:active:not(:disabled) { transform: translateY(1px); }

/* ────── 页面容器 ────── */
.page { flex: 1; display: flex; flex-direction: column; overflow: hidden; }
.page.scrollable { overflow-y: auto; }

/* ────── 导航 ────── */
.nav {
  display: flex; align-items: center; gap: 10px;
  margin-bottom: 14px; padding-bottom: 10px;
  border-bottom: 1px solid var(--border);
}

.btn-back {
  padding: 5px 12px;
  border: 1px solid var(--border-light);
  border-radius: 5px;
  background: transparent;
  color: var(--text-2);
  font-family: var(--font-display);
  font-size: 11px; font-weight: 600; letter-spacing: 0.5px;
  cursor: pointer; transition: all 0.15s ease;
  flex-shrink: 0;
}
.btn-back:hover { border-color: var(--primary); color: var(--text); }

.nav-title {
  font-family: var(--font-display);
  font-size: 15px; font-weight: 700; letter-spacing: 0.5px;
}

.nav-meta {
  color: var(--muted); font-size: 11px;
  margin-left: auto; font-variant-numeric: tabular-nums;
}

.approx-tip {
  font-size: 11px; color: var(--gold);
  margin-bottom: 10px; padding: 6px 10px;
  background: rgba(245, 158, 11, 0.08);
  border: 1px solid rgba(245, 158, 11, 0.2);
  border-radius: 6px;
}

/* ────── 输家高亮 ────── */
.loser-box {
  text-align: center;
  padding: 14px 12px; margin-bottom: 12px;
  background: linear-gradient(135deg, rgba(239,68,68,0.12) 0%, rgba(239,68,68,0.04) 100%);
  border-radius: 10px;
  border: 1px solid rgba(239, 68, 68, 0.3);
  box-shadow: 0 0 20px var(--danger-glow);
  position: relative; overflow: hidden;
}
.loser-box::before {
  content: ''; position: absolute; top: 0; left: 0; right: 0; height: 2px;
  background: linear-gradient(90deg, transparent, var(--danger), transparent);
}
.loser-name {
  font-family: var(--font-display);
  color: var(--danger); font-weight: 700;
  font-size: 16px; letter-spacing: 0.3px;
}
.loser-label { color: var(--danger); font-size: 13px; margin-left: 8px; opacity: 0.9; }

/* ────── 队伍卡片 ────── */
.team {
  background: var(--surface);
  border-radius: 8px; padding: 11px 13px; margin-bottom: 6px;
  border: 1px solid var(--border);
  transition: all 0.2s ease;
}
.team:hover { border-color: var(--border-light); }
.team.lose {
  border-color: rgba(239, 68, 68, 0.5);
  background: linear-gradient(135deg, rgba(239,68,68,0.05), var(--surface));
}

.team-top {
  display: flex; align-items: center;
  margin-bottom: 6px; padding-bottom: 6px;
  border-bottom: 1px solid var(--border);
}
.team-name {
  font-family: var(--font-display);
  font-weight: 600; font-size: 13px; flex: 1; letter-spacing: 0.3px;
}
.team-pts {
  font-family: var(--font-display);
  color: var(--primary-hi); font-weight: 700; font-size: 14px;
  font-variant-numeric: tabular-nums; letter-spacing: 0.5px;
}
.team.lose .team-pts { color: var(--danger); }

.p-row {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; padding: 3px 0;
}
.p-name {
  flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  color: var(--text-2);
}
.p-dmg { color: var(--text); width: 52px; text-align: right; font-variant-numeric: tabular-nums; font-weight: 500; }
.p-tkn { color: var(--gold); width: 52px; text-align: right; font-variant-numeric: tabular-nums; }
.p-kda { color: var(--muted); width: 50px; text-align: right; font-variant-numeric: tabular-nums; }

/* ────── 历史列表 ────── */
.h-list { flex: 1; overflow-y: auto; }
.h-list::-webkit-scrollbar { width: 4px; }
.h-list::-webkit-scrollbar-track { background: transparent; }
.h-list::-webkit-scrollbar-thumb { background: var(--border); border-radius: 2px; }

.h-item {
  display: flex; align-items: center; gap: 10px;
  padding: 11px 14px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 7px;
  margin-bottom: 5px;
  cursor: pointer; font-size: 12px;
  transition: all 0.15s ease;
}
.h-item:hover {
  border-color: var(--primary); background: var(--surface-2);
  transform: translateX(2px);
}
.h-mode { font-family: var(--font-display); font-weight: 600; flex: 1; color: var(--text); letter-spacing: 0.3px; }
.h-dur { color: var(--muted); font-variant-numeric: tabular-nums; }
.h-time { color: var(--muted); width: 62px; text-align: right; font-variant-numeric: tabular-nums; }
.h-arrow { color: var(--primary); font-size: 18px; font-weight: 600; }

/* ────── 设置页 ────── */
.section-label {
  font-family: var(--font-display);
  font-size: 11px; color: var(--muted);
  letter-spacing: 1px; text-transform: uppercase;
  margin-bottom: 8px;
}

.rule-list { display: flex; flex-direction: column; gap: 6px; }

.rule-item {
  display: flex; align-items: flex-start; gap: 10px;
  padding: 12px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.rule-item:hover { border-color: var(--border-light); }
.rule-item.selected {
  border-color: var(--primary);
  background: linear-gradient(135deg, rgba(124,58,237,0.08), var(--surface));
  box-shadow: 0 0 12px var(--primary-glow);
}
.rule-item input[type="radio"] {
  margin-top: 2px;
  accent-color: var(--primary);
  cursor: pointer;
}
.rule-body { flex: 1; }
.rule-name {
  font-family: var(--font-display);
  font-size: 13px; font-weight: 600;
  color: var(--text); margin-bottom: 3px;
  letter-spacing: 0.3px;
}
.rule-desc { font-size: 11px; color: var(--muted); line-height: 1.5; }

.about-box {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 8px; padding: 10px 12px;
}
.about-line { font-size: 12px; padding: 2px 0; color: var(--text-2); }
.about-line.muted { color: var(--muted); }
</style>
