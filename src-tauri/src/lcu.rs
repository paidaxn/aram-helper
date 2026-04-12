use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// LCU 连接信息
pub struct LcuConnection {
    port: String,
    auth: String,
    client: reqwest::Client,
}

/// 玩家数据
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

/// 分队结果
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TeamResult {
    pub name: String,
    pub players: Vec<PlayerDamage>,
    pub score: f64,
    pub is_loser: bool,
}

/// 整局游戏结果
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GameResult {
    pub all_players: Vec<PlayerDamage>,
    pub friend_count: usize,
    pub teams: Vec<TeamResult>,
    pub is_red_packet_game: bool,
}

impl LcuConnection {
    /// 连接到 LCU 客户端
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (port, token) = Self::find_credentials()?;

        let auth = base64::engine::general_purpose::STANDARD
            .encode(format!("riot:{}", token));

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self { port, auth, client })
    }

    /// 获取 LCU 端口和 token（全自动检测，无需配置路径）
    fn find_credentials() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        #[cfg(target_os = "windows")]
        {
            let mut errors = Vec::new();

            match Self::from_wmic() {
                Ok(result) => return Ok(result),
                Err(e) => errors.push(format!("wmic: {}", e)),
            }

            match Self::from_powershell_cim() {
                Ok(result) => return Ok(result),
                Err(e) => errors.push(format!("powershell: {}", e)),
            }

            match Self::from_process_lockfile() {
                Ok(result) => return Ok(result),
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
            let start = i + 11;
            let end = text[start..]
                .find(|c: char| !c.is_ascii_digit())
                .map(|j| start + j)
                .unwrap_or(text.len());
            let p = text[start..end].to_string();
            if p.is_empty() { None } else { Some(p) }
        })?;

        let token = text.find("--remoting-auth-token=").and_then(|i| {
            let start = i + 22;
            let end = text[start..]
                .find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .map(|j| start + j)
                .unwrap_or(text.len());
            let t = text[start..end].to_string();
            if t.is_empty() { None } else { Some(t) }
        })?;

        Some((port, token))
    }

    #[cfg(target_os = "windows")]
    fn from_wmic() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        let output = Command::new("cmd")
            .args(["/C", "wmic process where \"name='LeagueClientUx.exe'\" get CommandLine /value"])
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err("wmic: 未找到 LeagueClientUx.exe".into());
        }
        Self::extract_port_token(&stdout)
            .ok_or_else(|| "wmic: 命令行中未找到 port/token".into())
    }

    #[cfg(target_os = "windows")]
    fn from_powershell_cim() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command",
                "(Get-CimInstance Win32_Process -Filter \"name='LeagueClientUx.exe'\").CommandLine"])
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err("PowerShell: 未找到 LeagueClientUx.exe".into());
        }
        Self::extract_port_token(&stdout)
            .ok_or_else(|| "PowerShell: 命令行中未找到 port/token".into())
    }

    #[cfg(target_os = "windows")]
    fn from_process_lockfile() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;
        let output = Command::new("cmd")
            .args(["/C", "wmic process where \"name='LeagueClientUx.exe'\" get ExecutablePath /value"])
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let exe_path = stdout.lines()
            .find(|l| l.starts_with("ExecutablePath="))
            .map(|l| l.trim_start_matches("ExecutablePath=").trim())
            .unwrap_or("")
            .to_string();

        if exe_path.is_empty() {
            let output = Command::new("powershell")
                .args(["-NoProfile", "-Command",
                    "(Get-Process LeagueClientUx -ErrorAction SilentlyContinue).Path"])
                .output()?;
            let ps_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if ps_path.is_empty() {
                return Err("无法获取 LeagueClientUx 进程路径".into());
            }
            return Self::read_lockfile_from_exe(&ps_path);
        }
        Self::read_lockfile_from_exe(&exe_path)
    }

    #[cfg(target_os = "windows")]
    fn read_lockfile_from_exe(exe_path: &str) -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        let client_dir = std::path::Path::new(exe_path)
            .parent()
            .ok_or("无法获取客户端目录")?;
        let lockfile_path = client_dir.join("lockfile");
        let content = std::fs::read_to_string(&lockfile_path)
            .map_err(|e| format!("无法读取 lockfile {}: {}", lockfile_path.display(), e))?;
        let parts: Vec<&str> = content.trim().split(':').collect();
        if parts.len() >= 4 {
            Ok((parts[2].to_string(), parts[3].to_string()))
        } else {
            Err(format!("lockfile 格式异常: {}", content).into())
        }
    }

    /// 调用 LCU API
    async fn api(&self, path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("https://127.0.0.1:{}{}", self.port, path);
        let res = self.client
            .get(&url)
            .header("Authorization", format!("Basic {}", self.auth))
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(format!("API {} 返回 {}", path, res.status()).into());
        }

        Ok(res.json().await?)
    }

    /// 获取当前召唤师名称
    pub async fn get_current_summoner(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.api("/lol-summoner/v1/current-summoner").await?;
        Ok(data["displayName"].as_str().unwrap_or("未知").to_string())
    }

    /// 获取当前游戏流程阶段
    pub async fn get_gameflow_phase(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.api("/lol-gameflow/v1/gameflow-phase").await?;
        Ok(data.as_str().unwrap_or("None").to_string())
    }

    /// 获取好友 PUUID 列表
    async fn get_friend_puuids(&self) -> Result<HashSet<String>, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.api("/lol-chat/v1/friends").await?;
        let puuids: HashSet<String> = data
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|f| f["puuid"].as_str().map(String::from))
            .collect();
        Ok(puuids)
    }

    /// 获取最近一局的红包局结果
    pub async fn get_last_game_result(&self) -> Result<GameResult, Box<dyn std::error::Error + Send + Sync>> {
        // 获取当前召唤师 PUUID
        let summoner = self.api("/lol-summoner/v1/current-summoner").await?;
        let my_puuid = summoner["puuid"]
            .as_str()
            .ok_or("无法获取 PUUID")?
            .to_string();

        // 获取好友列表
        let friend_puuids = self.get_friend_puuids().await.unwrap_or_default();

        // 获取最近一局
        let history = self.api(
            &format!("/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex=1", my_puuid)
        ).await?;

        let game_id = history["games"]["games"][0]["gameId"]
            .as_i64()
            .ok_or("没有找到最近的对局记录")?;

        // 获取对局详情
        let details = self.api(&format!("/lol-match-history/v1/games/{}", game_id)).await?;

        let participants = details["participants"]
            .as_array()
            .ok_or("对局详情中没有参与者数据")?;

        let identities = details["participantIdentities"]
            .as_array()
            .ok_or("对局详情中没有玩家身份数据")?;

        // 构建 participantId -> 玩家信息的映射
        let mut name_map: HashMap<i64, String> = HashMap::new();
        let mut puuid_map: HashMap<i64, String> = HashMap::new();
        for identity in identities {
            let pid = identity["participantId"].as_i64().unwrap_or(0);
            let name = identity["player"]["summonerName"]
                .as_str()
                .unwrap_or("未知")
                .to_string();
            let player_puuid = identity["player"]["puuid"]
                .as_str()
                .unwrap_or("")
                .to_string();
            name_map.insert(pid, name);
            puuid_map.insert(pid, player_puuid);
        }

        // 找到当前玩家的队伍
        let my_participant_id = puuid_map
            .iter()
            .find(|(_, v)| v.as_str() == my_puuid)
            .map(|(k, _)| *k)
            .ok_or("无法在对局中找到当前玩家")?;

        let my_team_id = participants
            .iter()
            .find(|p| p["participantId"].as_i64().unwrap_or(0) == my_participant_id)
            .and_then(|p| p["teamId"].as_i64())
            .ok_or("无法确定当前玩家的队伍")?;

        // 取己方队伍，按 participantId 排序（对应楼层）
        let mut team_participants: Vec<&serde_json::Value> = participants
            .iter()
            .filter(|p| p["teamId"].as_i64().unwrap_or(0) == my_team_id)
            .collect();
        team_participants.sort_by_key(|p| p["participantId"].as_i64().unwrap_or(0));

        // 构建玩家数据，计算分数，标记好友
        let all_players: Vec<PlayerDamage> = team_participants
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let pid = p["participantId"].as_i64().unwrap_or(0);
                let player_puuid = puuid_map.get(&pid).cloned().unwrap_or_default();
                let damage = p["stats"]["totalDamageDealtToChampions"].as_i64().unwrap_or(0);
                let damage_taken = p["stats"]["totalDamageTaken"].as_i64().unwrap_or(0);
                let is_me = player_puuid == my_puuid;
                let is_friend = is_me || friend_puuids.contains(&player_puuid);

                PlayerDamage {
                    summoner_name: name_map.get(&pid).cloned().unwrap_or_else(|| "未知".to_string()),
                    champion_id: p["championId"].as_i64().unwrap_or(0),
                    damage,
                    damage_taken,
                    score: damage as f64 + damage_taken as f64 / 2.0,
                    kills: p["stats"]["kills"].as_i64().unwrap_or(0),
                    deaths: p["stats"]["deaths"].as_i64().unwrap_or(0),
                    assists: p["stats"]["assists"].as_i64().unwrap_or(0),
                    floor: i + 1,
                    is_friend,
                }
            })
            .collect();

        // 筛选出好友玩家
        let friends: Vec<&PlayerDamage> = all_players.iter().filter(|p| p.is_friend).collect();
        let friend_count = friends.len();

        // 根据好友人数分组
        let teams = Self::group_into_teams(&friends, friend_count);
        let is_red_packet_game = friend_count >= 2;

        Ok(GameResult {
            all_players,
            friend_count,
            teams,
            is_red_packet_game,
        })
    }

    /// 根据好友人数和楼层分组
    fn group_into_teams(friends: &[&PlayerDamage], count: usize) -> Vec<TeamResult> {
        if count < 2 {
            return vec![];
        }

        // 按楼层排序
        let mut sorted: Vec<PlayerDamage> = friends.iter().map(|p| (*p).clone()).collect();
        sorted.sort_by_key(|p| p.floor);

        let mut teams = match count {
            // 2-3人：每人单独一队
            2 | 3 => {
                sorted.iter().map(|p| {
                    TeamResult {
                        name: format!("{}楼 {}", p.floor, p.summoner_name),
                        players: vec![p.clone()],
                        score: p.score,
                        is_loser: false,
                    }
                }).collect::<Vec<_>>()
            }
            // 4人：前2 vs 后2
            4 => {
                let s1 = sorted[0].score + sorted[1].score;
                let s2 = sorted[2].score + sorted[3].score;
                vec![
                    TeamResult {
                        name: format!("{}+{}楼", sorted[0].floor, sorted[1].floor),
                        players: vec![sorted[0].clone(), sorted[1].clone()],
                        score: s1,
                        is_loser: false,
                    },
                    TeamResult {
                        name: format!("{}+{}楼", sorted[2].floor, sorted[3].floor),
                        players: vec![sorted[2].clone(), sorted[3].clone()],
                        score: s2,
                        is_loser: false,
                    },
                ]
            }
            // 5人：1+2楼 / 3楼(×2补偿) / 4+5楼
            5 => {
                let s1 = sorted[0].score + sorted[1].score;
                // 3楼单独，伤害×2 + 承伤（等效两人）
                let s2 = sorted[2].damage as f64 * 2.0 + sorted[2].damage_taken as f64;
                let s3 = sorted[3].score + sorted[4].score;
                vec![
                    TeamResult {
                        name: format!("{}+{}楼", sorted[0].floor, sorted[1].floor),
                        players: vec![sorted[0].clone(), sorted[1].clone()],
                        score: s1,
                        is_loser: false,
                    },
                    TeamResult {
                        name: format!("{}楼(×2)", sorted[2].floor),
                        players: vec![sorted[2].clone()],
                        score: s2,
                        is_loser: false,
                    },
                    TeamResult {
                        name: format!("{}+{}楼", sorted[3].floor, sorted[4].floor),
                        players: vec![sorted[3].clone(), sorted[4].clone()],
                        score: s3,
                        is_loser: false,
                    },
                ]
            }
            _ => vec![],
        };

        // 标记分数最低的队伍为输家
        if let Some(min_score) = teams.iter().map(|t| t.score).reduce(f64::min) {
            for team in &mut teams {
                if (team.score - min_score).abs() < 0.01 {
                    team.is_loser = true;
                    break; // 只标记一个
                }
            }
        }

        teams
    }
}
