use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LCU 连接信息
pub struct LcuConnection {
    port: String,
    auth: String,
    client: reqwest::Client,
}

/// 玩家伤害数据（返回给前端）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDamage {
    pub summoner_name: String,
    pub champion_id: i64,
    pub team_id: i64,
    pub damage: i64,
    pub kills: i64,
    pub deaths: i64,
    pub assists: i64,
    pub is_lowest: bool,
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
        // 方式1：从进程命令行参数获取 port 和 token（最可靠）
        #[cfg(target_os = "windows")]
        {
            if let Ok(result) = Self::from_process_args() {
                return Ok(result);
            }
        }

        // 方式2：从进程路径定位 lockfile（备用）
        #[cfg(target_os = "windows")]
        {
            if let Ok(result) = Self::from_process_lockfile() {
                return Ok(result);
            }
        }

        Err("找不到英雄联盟客户端，请确保游戏已启动".into())
    }

    /// 从命令行参数中提取 port 和 token
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

    /// Windows: 通过 PowerShell 获取进程命令行参数（兼容 Win10/11）
    #[cfg(target_os = "windows")]
    fn from_process_args() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;

        let output = Command::new("powershell")
            .args([
                "-NoProfile", "-Command",
                "(Get-CimInstance Win32_Process -Filter \"name='LeagueClientUx.exe'\").CommandLine",
            ])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return Err("LeagueClientUx.exe 进程未运行".into());
        }

        Self::extract_port_token(&stdout)
            .ok_or_else(|| "进程命令行中未找到 port/token".into())
    }

    /// Windows: 通过进程路径定位 lockfile（备用方案）
    #[cfg(target_os = "windows")]
    fn from_process_lockfile() -> Result<(String, String), Box<dyn std::error::Error + Send + Sync>> {
        use std::process::Command;

        // 获取 LeagueClientUx.exe 的可执行文件路径
        let output = Command::new("powershell")
            .args([
                "-NoProfile", "-Command",
                "(Get-Process LeagueClientUx -ErrorAction SilentlyContinue).Path",
            ])
            .output()?;

        let exe_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if exe_path.is_empty() {
            return Err("未找到 LeagueClientUx 进程".into());
        }

        // lockfile 和客户端 exe 在同一目录
        let client_dir = std::path::Path::new(&exe_path)
            .parent()
            .ok_or("无法获取客户端目录")?;
        let lockfile_path = client_dir.join("lockfile");

        let content = std::fs::read_to_string(&lockfile_path)
            .map_err(|_| format!("无法读取 lockfile: {}", lockfile_path.display()))?;
        let parts: Vec<&str> = content.trim().split(':').collect();
        if parts.len() >= 4 {
            Ok((parts[2].to_string(), parts[3].to_string()))
        } else {
            Err("lockfile 格式异常".into())
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
        let name = data["displayName"].as_str().unwrap_or("未知").to_string();
        Ok(name)
    }

    /// 获取当前游戏流程阶段
    pub async fn get_gameflow_phase(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.api("/lol-gameflow/v1/gameflow-phase").await?;
        Ok(data.as_str().unwrap_or("None").to_string())
    }

    /// 获取最近一局的伤害排名（仅己方队伍）
    pub async fn get_last_game_damage(&self) -> Result<Vec<PlayerDamage>, Box<dyn std::error::Error + Send + Sync>> {
        // 获取当前召唤师 PUUID
        let summoner = self.api("/lol-summoner/v1/current-summoner").await?;
        let puuid = summoner["puuid"]
            .as_str()
            .ok_or("无法获取 PUUID")?
            .to_string();

        // 获取最近一局
        let history = self.api(
            &format!("/lol-match-history/v1/products/lol/{}/matches?begIndex=0&endIndex=1", puuid)
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

        // 构建 participantId -> 玩家名和 puuid 的映射
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

        // 找到当前玩家的 participantId 和 teamId
        let my_participant_id = puuid_map
            .iter()
            .find(|(_, v)| v.as_str() == puuid)
            .map(|(k, _)| *k)
            .ok_or("无法在对局中找到当前玩家")?;

        let my_team_id = participants
            .iter()
            .find(|p| p["participantId"].as_i64().unwrap_or(0) == my_participant_id)
            .and_then(|p| p["teamId"].as_i64())
            .ok_or("无法确定当前玩家的队伍")?;

        // 只取己方队伍的 5 人数据
        let mut players: Vec<PlayerDamage> = participants
            .iter()
            .filter(|p| p["teamId"].as_i64().unwrap_or(0) == my_team_id)
            .map(|p| {
                let pid = p["participantId"].as_i64().unwrap_or(0);
                PlayerDamage {
                    summoner_name: name_map.get(&pid).cloned().unwrap_or_else(|| "未知".to_string()),
                    champion_id: p["championId"].as_i64().unwrap_or(0),
                    team_id: my_team_id,
                    damage: p["stats"]["totalDamageDealtToChampions"].as_i64().unwrap_or(0),
                    kills: p["stats"]["kills"].as_i64().unwrap_or(0),
                    deaths: p["stats"]["deaths"].as_i64().unwrap_or(0),
                    assists: p["stats"]["assists"].as_i64().unwrap_or(0),
                    is_lowest: false,
                }
            })
            .collect();

        // 按伤害降序排序（最高在前）
        players.sort_by(|a, b| b.damage.cmp(&a.damage));

        // 标记伤害最低的玩家
        if let Some(last) = players.last_mut() {
            last.is_lowest = true;
        }

        Ok(players)
    }
}
