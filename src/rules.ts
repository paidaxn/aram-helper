// 红包局规则引擎 - 可扩展的多规则支持

export interface Player {
  summonerName: string;
  championId: number;
  damage: number;
  damageTaken: number;
  goldEarned: number;
  kills: number;
  deaths: number;
  assists: number;
  floor: number;
  isFriend: boolean;
  isMe: boolean;
}

export interface GameResult {
  gameId: number;
  gameMode: string;
  gameDuration: number;
  timestamp: number;
  players: Player[];
  hasAccurateFloors: boolean;
}

export interface Team {
  name: string;
  players: Player[];
  score: number;
  isLoser: boolean;
}

export interface RuleResult {
  teams: Team[];
  friendCount: number;
  isRedPacketGame: boolean;
  scoreLabel: string; // "综合分" / "伤害" / "OP 分"
  needsFloors: boolean; // 该规则是否依赖楼层数据
}

export interface ScoringRule {
  id: string;
  name: string;
  description: string;
  calculate(game: GameResult): RuleResult;
}

// 找最低分标为输家（支持多个并列）
function markLoser(teams: Team[]) {
  if (teams.length === 0) return;
  const min = Math.min(...teams.map((t) => t.score));
  for (const t of teams) {
    if (Math.abs(t.score - min) < 0.01) {
      t.isLoser = true;
      break; // 只标一个（并列时取靠前的）
    }
  }
}

// ────── 规则 1：红包局经典 ──────
const classicRule: ScoringRule = {
  id: "classic",
  name: "红包局经典",
  description: "输出+承伤/2，按选英雄楼层分组。2-3人每人单独；4人前2后2；5人1+2楼/3楼(×2补偿)/4+5楼",
  calculate(game) {
    const friends = game.players.filter((p) => p.isFriend);
    const count = friends.length;
    const label = "综合分";

    if (count < 2) {
      return { teams: [], friendCount: count, isRedPacketGame: false, scoreLabel: label, needsFloors: true };
    }

    const sorted = [...friends].sort((a, b) => a.floor - b.floor);
    const score = (p: Player) => p.damage + p.damageTaken / 2;
    const soloScore = (p: Player) => p.damage * 2 + p.damageTaken;

    let teams: Team[] = [];

    if (count === 2 || count === 3) {
      teams = sorted.map((p) => ({
        name: p.summonerName,
        players: [p],
        score: score(p),
        isLoser: false,
      }));
    } else if (count === 4) {
      teams = [
        {
          name: `${sorted[0].summonerName}、${sorted[1].summonerName}`,
          players: [sorted[0], sorted[1]],
          score: score(sorted[0]) + score(sorted[1]),
          isLoser: false,
        },
        {
          name: `${sorted[2].summonerName}、${sorted[3].summonerName}`,
          players: [sorted[2], sorted[3]],
          score: score(sorted[2]) + score(sorted[3]),
          isLoser: false,
        },
      ];
    } else if (count === 5) {
      teams = [
        {
          name: `${sorted[0].summonerName}、${sorted[1].summonerName}`,
          players: [sorted[0], sorted[1]],
          score: score(sorted[0]) + score(sorted[1]),
          isLoser: false,
        },
        {
          name: `${sorted[2].summonerName}（单人×2）`,
          players: [sorted[2]],
          score: soloScore(sorted[2]),
          isLoser: false,
        },
        {
          name: `${sorted[3].summonerName}、${sorted[4].summonerName}`,
          players: [sorted[3], sorted[4]],
          score: score(sorted[3]) + score(sorted[4]),
          isLoser: false,
        },
      ];
    }

    markLoser(teams);
    return { teams, friendCount: count, isRedPacketGame: true, scoreLabel: label, needsFloors: true };
  },
};

// ────── 规则 2：伤害最低 ──────
const lowestDamageRule: ScoringRule = {
  id: "lowest-damage",
  name: "纯伤害最低",
  description: "谁对英雄输出最低谁发红包，无分组",
  calculate(game) {
    const friends = game.players.filter((p) => p.isFriend);
    const count = friends.length;

    if (count < 2) {
      return { teams: [], friendCount: count, isRedPacketGame: false, scoreLabel: "伤害", needsFloors: false };
    }

    const teams: Team[] = friends.map((p) => ({
      name: p.summonerName,
      players: [p],
      score: p.damage,
      isLoser: false,
    }));

    markLoser(teams);
    return { teams, friendCount: count, isRedPacketGame: true, scoreLabel: "伤害", needsFloors: false };
  },
};

// ────── 规则 3：OP 综合评分（简化版 OPGG） ──────
const opgRule: ScoringRule = {
  id: "op-score",
  name: "OP 综合评分",
  description: "模拟 OPGG 评分逻辑：综合伤害、承伤、经济、KDA。分数越低表现越差",
  calculate(game) {
    const friends = game.players.filter((p) => p.isFriend);
    const count = friends.length;

    if (count < 2) {
      return { teams: [], friendCount: count, isRedPacketGame: false, scoreLabel: "OP 分", needsFloors: false };
    }

    // 用全队 5 人作为基准（更公平）
    const all = game.players;
    const avg = (key: keyof Player) =>
      all.reduce((s, p) => s + (p[key] as number), 0) / all.length || 1;

    const avgDmg = avg("damage");
    const avgTkn = avg("damageTaken");
    const avgGold = avg("goldEarned");

    const calcOP = (p: Player) => {
      const dmgRatio = p.damage / avgDmg;
      const tknRatio = p.damageTaken / avgTkn;
      const goldRatio = p.goldEarned / avgGold;
      const kdaBonus =
        (p.kills * 1.0 + p.assists * 0.7 - p.deaths * 0.8) / 10;
      // 0-10 分范围：权重归一化
      const raw = dmgRatio * 4 + tknRatio * 2 + goldRatio * 2 + kdaBonus;
      return Math.max(0, Math.min(10, raw));
    };

    const teams: Team[] = friends.map((p) => ({
      name: p.summonerName,
      players: [p],
      score: calcOP(p),
      isLoser: false,
    }));

    markLoser(teams);
    return { teams, friendCount: count, isRedPacketGame: true, scoreLabel: "OP 分", needsFloors: false };
  },
};

// ────── 导出规则列表 ──────
export const RULES: ScoringRule[] = [classicRule, lowestDamageRule, opgRule];

export function getRule(id: string): ScoringRule {
  return RULES.find((r) => r.id === id) || RULES[0];
}
