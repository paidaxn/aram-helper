use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub struct LcuConnection {
    port: String,
    auth: String,
    client: reqwest::Client,
}

// ────── 数据结构 ──────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDamage {
    pub summoner_name: String,
    pub champion_id: i64,
    pub damage: i64,
    pub damage_taken: i64,
    pub score: f64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub floor: usize,
    pub is_friend: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TeamResult {
    pub name: String,
    pub players: Vec<PlayerDamage>,
    pub score: f64,
    pub is_loser: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameResult {
    pub game_id: i64,
    pub game_mode: String,
    pub game_duration: i64,
    pub timestamp: i64,
    pub all_players: Vec<PlayerDamage>,
    pub friend_count: usize,
    pub teams: Vec<TeamResult>,
    pub is_red_packet_game: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchSummary {
    pub game_id: i64,
    pub game_mode: String,
    pub game_duration: i64,
    pub timestamp: i64,
    pub champion_id: i64,
}

// ────── 算分规则（可扩展） ──────

/// 当前规则：输出伤害 + 承受伤害 / 2
fn calculate_score(damage: i64, damage_taken: i64) -> f64 {
    damage as f64 + damage_taken as f64 / 2.0
}

/// 5 人模式中 3 楼（单人队）的等效分数：输出×2 + 承伤
fn calculate_solo_team_score(player: &PlayerDamage) -> f64 {
    player.damage as f64 * 2.0 + player.damage_taken as f64
}

// ────── 分组逻辑（可扩展） ──────

fn group_into_teams(friends: &[&PlayerDamage]) -> Vec<TeamResult> {
    let count = friends.len();
    if count < 2 {
        return vec![];
    }

    let mut sorted: Vec<PlayerDamage> = friends.iter().map(|p| (*p).clone()).collect();
    sorted.sort_by_key(|p| p.floor);

    let mut teams = match count {
        2 | 3 => {
            sorted.iter().map(|p| TeamResult {
                name: p.summoner_name.clone(),
                players: vec![p.clone()],
                score: p.score,
                is_loser: false,
            }).collect::<Vec<_>>()
        }
        4 => {
            let s1 = sorted[0].score + sorted[1].score;
            let s2 = sorted[2].score + sorted[3].score;
            vec![
                TeamResult {
                    name: format!("{}、{}", sorted[0].summoner_name, sorted[1].summoner_name),
                    players: vec![sorted[0].clone(), sorted[1].clone()],
                    score: s1,
                    is_loser: false,
                },
                TeamResult {
                    name: format!("{}、{}", sorted[2].summoner_name, sorted[3].summoner_name),
                    players: vec![sorted[2].clone(), sorted[3].clone()],
                    score: s2,
                    is_loser: false,
                },
            ]
        }
        5 => {
            let s1 = sorted[0].score + sorted[1].score;
            let s2 = calculate_solo_team_score(&sorted[2]);
            let s3 = sorted[3].score + sorted[4].score;
            vec![
                TeamResult {
                    name: format!("{}、{}", sorted[0].summoner_name, sorted[1].summoner_name),
                    players: vec![sorted[0].clone(), sorted[1].clone()],
                    score: s1,
                    is_loser: false,
                },
                TeamResult {
                    name: format!("{}（单人×2）", sorted[2].summoner_name),
                    players: vec![sorted[2].clone()],
                    score: s2,
                    is_loser: false,
                },
                TeamResult {
                    name: format!("{}、{}", sorted[3].summoner_name, sorted[4].summoner_name),
                    players: vec![sorted[3].clone(), sorted[4].clone()],
                    score: s3,
                    is_loser: false,
                },
            ]
        }
        _ => vec![],
    };

    // 标记分数最低的队伍
    if let Some(min_score) = teams.iter().map(|t| t.score).reduce(f64::min) {
        for team in &mut teams {
            if (team.score - min_score).abs() < 0.01 {
                team.is_loser = true;
                break;
            }
        }
    }

    teams
}

// ────── LCU 连接 ──────

impl LcuConnection {
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (port, token) = Self::find_credentials()?;
        let auth = base64::engine::general_purpose::STANDARD.encode(format!("riot:{}", token));
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;
        Ok(Self { port, auth, client })
    }

    fn find_credentials() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_os = "windows")]
        {
            let mut errors = Vec::new();
            match Self::from_wmic() {
                Ok(r) => return Ok(r),
                Err(e) => errors.push(format!("wmic: {}", e)),
            }
            match Self::from_powershell_cim() {
                Ok(r) => return Ok(r),
                Err(e) => errors.push(format!("powershell: {}", e)),
            }
            match Self::from_process_lockfile() {
                Ok(r) => return Ok(r),
                Err(e) => errors.push(format!("lockfile: {}", e)),
            }
            return Err(format!("所有检测方式均失败:\n{}", errors.join("\n")).into());
        }
        #[cfg(not(target_os = "windows"))]
        Err("仅支持 Windows 系统".into())
    }

    #[cfg(target_os = "windows")]
    fn extract_port_token(text: &str) -> Option<(String, String)> {
        let port = text.find("--app-port=").and_then(|i| {
            let s = i + 11;
            let e = text[s..].find(|c: char| !c.is_ascii_digit()).map(|j| s + j).unwrap_or(text.len());
            let p = text[s..e].to_string();
            if p.is_empty() { None } else { Some(p) }
        })?;
        let token = text.find("--remoting-auth-token=").and_then(|i| {
            let s = i + 22;
            let e = text[s..].find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_').map(|j| s + j).unwrap_or(text.len());
            let t = text[s..e].to_string();
            if t.is_empty() { None } else { Some(t) }
        })?;
        Some((port, token))
    }

    #[cfg(target_os = "windows")]
    fn from_wmic() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        let out = Command::new("cmd")
            .args(["/C", "wmic process where \"name='LeagueClientUx.exe'\" get CommandLine /value"])
            .output()?;
        let s = String::from_utf8_lossy(&out.stdout);
        if s.trim().is_empty() { return Err("未找到 LeagueClientUx.exe".into()); }
        Self::extract_port_token(&s).ok_or_else(|| "命令行中未找到 port/token".into())
    }

    #[cfg(target_os = "windows")]
    fn from_powershell_cim() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        let out = Command::new("powershell")
            .args(["-NoProfile", "-Command", "(Get-CimInstance Win32_Process -Filter \"name='LeagueClientUx.exe'\").CommandLine"])
            .output()?;
        let s = String::from_utf8_lossy(&out.stdout);
        if s.trim().is_empty() { return Err("未找到 LeagueClientUx.exe".into()); }
        Self::extract_port_token(&s).ok_or_else(|| "命令行中未找到 port/token".into())
    }

    #[cfg(target_os = "windows")]
    fn from_process_lockfile() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        let out = Command::new("cmd")
            .args(["/C", "wmic process where \"name='LeagueClientUx.exe'\" get ExecutablePath /value"])
            .output()?;
        let s = String::from_utf8_lossy(&out.stdout);
        let path = s.lines()
            .find(|l| l.starts_with("ExecutablePath="))
            .map(|l| l.trim_start_matches("ExecutablePath=").trim().to_string())
            .unwrap_or_default();
        if path.is_empty() {
            let out = Command::new("powershell")
                .args(["-NoProfile", "-Command", "(Get-Process LeagueClientUx -ErrorAction SilentlyContinue).Path"])
                .output()?;
            let ps = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if ps.is_empty() { return Err("无法获取进程路径".into()); }
            return Self::read_lockfile_from_exe(&ps);
        }
        Self::read_lockfile_from_exe(&path)
    }

    #[cfg(target_os = "windows")]
    fn read_lockfile_from_exe(exe: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        let dir = std::path::Path::new(exe).parent().ok_or("无法获取目录")?;
        let content = std::fs::read_to_string(dir.join("lockfile"))
            .map_err(|e| format!("lockfile: {}", e))?;
        let parts: Vec<&str> = content.trim().split(':').collect();
        if parts.len() >= 4 { Ok((parts[2].to_string(), parts[3].to_string())) }
        else { Err(format!("lockfile 格式异常: {}", content).into()) }
    }

    // ────── API 调用 ──────

    async fn api(&self, path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("https://127.0.0.1:{}{}", self.port, path);
        let res = self.client.get(&url)
            .header("Authorization", format!("Basic {}", self.auth))
            .send().await?;
        if !res.status().is_success() {
            return Err(format!("API {} 返回 {}", path, res.status()).into());
        }
        Ok(res.json().await?)
    }

    pub async fn get_current_summoner(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.api("/lol-summoner/v1/current-summoner").await?;
        Ok(data["displayName"].as_str().unwrap_or("未知").to_string())
    }

    pub async fn get_gameflow_phase(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.api("/lol-gameflow/v1/gameflow-phase").await?;
        Ok(data.as_str().unwrap_or("None").to_string())
    }

    async fn get_friend_puuids(&self) -> HashSet<String> {
        self.api("/lol-chat/v1/friends").await
            .map(|data| {
                data.as_array().unwrap_or(&vec![]).iter()
                    .filter_map(|f| f["puuid"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 获取最近 N 局的概要列表
    pub async fn get_match_list(&self, count: usize) -> Result<Vec<MatchSummary>, Box<dyn std::error::Error + Send + Sync>> {
        let summoner = self.api("/lol-summoner/v1/current-summoner").await?;
        let puuid = summoner["puuid"].as_str().ok_or("无法获取 PUUID")?;

        let history = self.api(
            &format!("/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex={}", puuid, count)
        ).await?;

        let games = history["games"]["games"].as_array().ok_or("无对局记录")?;

        let mut list = Vec::new();
        for game in games {
            // 找到当前玩家使用的英雄
            let my_champion = Self::find_my_champion(game, puuid);
            list.push(MatchSummary {
                game_id: game["gameId"].as_i64().unwrap_or(0),
                game_mode: game["gameMode"].as_str().unwrap_or("").to_string(),
                game_duration: game["gameDuration"].as_i64().unwrap_or(0),
                timestamp: game["gameCreation"].as_i64().unwrap_or(0),
                champion_id: my_champion,
            });
        }

        Ok(list)
    }

    fn find_my_champion(game: &serde_json::Value, puuid: &str) -> i64 {
        let identities = game["participantIdentities"].as_array();
        let participants = game["participants"].as_array();

        if let (Some(ids), Some(parts)) = (identities, participants) {
            for id in ids {
                if id["player"]["puuid"].as_str() == Some(puuid) {
                    let pid = id["participantId"].as_i64().unwrap_or(0);
                    for p in parts {
                        if p["participantId"].as_i64() == Some(pid) {
                            return p["championId"].as_i64().unwrap_or(0);
                        }
                    }
                }
            }
        }
        0
    }

    /// 获取指定对局的红包局结果
    pub async fn get_game_result(&self, game_id: i64) -> Result<GameResult, Box<dyn std::error::Error + Send + Sync>> {
        let summoner = self.api("/lol-summoner/v1/current-summoner").await?;
        let my_puuid = summoner["puuid"].as_str().ok_or("无法获取 PUUID")?.to_string();

        let friend_puuids = self.get_friend_puuids().await;

        let details = self.api(&format!("/lol-match-history/v1/games/{}", game_id)).await?;

        let game_mode = details["gameMode"].as_str().unwrap_or("").to_string();
        let game_duration = details["gameDuration"].as_i64().unwrap_or(0);
        let timestamp = details["gameCreation"].as_i64().unwrap_or(0);

        let participants = details["participants"].as_array().ok_or("无参与者数据")?;
        let identities = details["participantIdentities"].as_array().ok_or("无身份数据")?;

        // 构建映射
        let mut name_map: HashMap<i64, String> = HashMap::new();
        let mut puuid_map: HashMap<i64, String> = HashMap::new();
        for id in identities {
            let pid = id["participantId"].as_i64().unwrap_or(0);
            name_map.insert(pid, id["player"]["summonerName"].as_str().unwrap_or("未知").to_string());
            puuid_map.insert(pid, id["player"]["puuid"].as_str().unwrap_or("").to_string());
        }

        // 找到己方队伍
        let my_pid = puuid_map.iter()
            .find(|(_, v)| v.as_str() == my_puuid)
            .map(|(k, _)| *k)
            .ok_or("无法找到当前玩家")?;

        let my_team_id = participants.iter()
            .find(|p| p["participantId"].as_i64() == Some(my_pid))
            .and_then(|p| p["teamId"].as_i64())
            .ok_or("无法确定队伍")?;

        // 己方队员按 participantId 排序（对应楼层）
        let mut team: Vec<&serde_json::Value> = participants.iter()
            .filter(|p| p["teamId"].as_i64() == Some(my_team_id))
            .collect();
        team.sort_by_key(|p| p["participantId"].as_i64().unwrap_or(0));

        let all_players: Vec<PlayerDamage> = team.iter().enumerate().map(|(i, p)| {
            let pid = p["participantId"].as_i64().unwrap_or(0);
            let player_puuid = puuid_map.get(&pid).cloned().unwrap_or_default();
            let damage = p["stats"]["totalDamageDealtToChampions"].as_i64().unwrap_or(0);
            let damage_taken = p["stats"]["totalDamageTaken"].as_i64().unwrap_or(0);
            let is_me = player_puuid == my_puuid;

            PlayerDamage {
                summoner_name: name_map.get(&pid).cloned().unwrap_or("未知".to_string()),
                champion_id: p["championId"].as_i64().unwrap_or(0),
                damage,
                damage_taken,
                score: calculate_score(damage, damage_taken),
                kills: p["stats"]["kills"].as_i64().unwrap_or(0),
                deaths: p["stats"]["deaths"].as_i64().unwrap_or(0),
                assists: p["stats"]["assists"].as_i64().unwrap_or(0),
                floor: i + 1,
                is_friend: is_me || friend_puuids.contains(&player_puuid),
            }
        }).collect();

        let friends: Vec<&PlayerDamage> = all_players.iter().filter(|p| p.is_friend).collect();
        let friend_count = friends.len();
        let teams = group_into_teams(&friends);
        let is_red_packet_game = friend_count >= 2;

        Ok(GameResult {
            game_id,
            game_mode,
            game_duration,
            timestamp,
            all_players,
            friend_count,
            teams,
            is_red_packet_game,
        })
    }

    /// 获取最近一局的红包局结果
    pub async fn get_last_game_result(&self) -> Result<GameResult, Box<dyn std::error::Error + Send + Sync>> {
        let summoner = self.api("/lol-summoner/v1/current-summoner").await?;
        let puuid = summoner["puuid"].as_str().ok_or("无法获取 PUUID")?;

        let history = self.api(
            &format!("/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex=1", puuid)
        ).await?;

        let game_id = history["games"]["games"][0]["gameId"]
            .as_i64()
            .ok_or("没有找到最近的对局记录")?;

        self.get_game_result(game_id).await
    }
}
