use base64::Engine;
use serde::{Deserialize, Serialize};
use std::process::Command;

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
    pub async fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        let (port, token) = Self::find_credentials()?;

        let auth = base64::engine::general_purpose::STANDARD
            .encode(format!("riot:{}", token));

        // 创建忽略自签名证书的 HTTP 客户端
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self { port, auth, client })
    }

    /// 获取 LCU 端口和 token（三级降级策略）
    fn find_credentials() -> Result<(String, String), Box<dyn std::error::Error>> {
        // 方式1：从进程命令行获取（Windows）
        #[cfg(target_os = "windows")]
        {
            if let Ok((port, token)) = Self::from_process_args() {
                return Ok((port, token));
            }
        }

        // 方式2：读 lockfile
        let lockfile_paths = [
            "E:/game/WeGameApps/英雄联盟/LeagueClient/lockfile",
            "C:/Riot Games/League of Legends/lockfile",
            "D:/Riot Games/League of Legends/lockfile",
            "E:/Riot Games/League of Legends/lockfile",
            "D:/英雄联盟/LeagueClient/lockfile",
            "E:/英雄联盟/LeagueClient/lockfile",
        ];

        for path in &lockfile_paths {
            if let Ok(content) = std::fs::read_to_string(path) {
                let content = content.trim().to_string();
                if content.contains(':') {
                    let parts: Vec<&str> = content.split(':').collect();
                    if parts.len() >= 4 {
                        return Ok((parts[2].to_string(), parts[3].to_string()));
                    }
                }
            }
        }

        Err("找不到英雄联盟客户端，请确保游戏已启动".into())
    }

    /// Windows: 从进程命令行参数获取端口和 token
    #[cfg(target_os = "windows")]
    fn from_process_args() -> Result<(String, String), Box<dyn std::error::Error>> {
        let output = Command::new("wmic")
            .args(["process", "where", "name='LeagueClientUx.exe'", "get", "CommandLine", "/format:list"])
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let port = stdout
            .find("--app-port=")
            .and_then(|i| {
                let start = i + 11;
                let end = stdout[start..].find(|c: char| !c.is_ascii_digit()).map(|j| start + j).unwrap_or(stdout.len());
                Some(stdout[start..end].to_string())
            })
            .ok_or("未找到 app-port")?;

        let token = stdout
            .find("--remoting-auth-token=")
            .and_then(|i| {
                let start = i + 22;
                let end = stdout[start..].find(|c: char| !c.is_alphanumeric() && c != '-' && c != '_').map(|j| start + j).unwrap_or(stdout.len());
                Some(stdout[start..end].to_string())
            })
            .ok_or("未找到 auth-token")?;

        Ok((port, token))
    }

    /// 调用 LCU API
    async fn api(&self, path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
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
    pub async fn get_current_summoner(&self) -> Result<String, Box<dyn std::error::Error>> {
        let data = self.api("/lol-summoner/v1/current-summoner").await?;
        let name = data["displayName"].as_str().unwrap_or("未知").to_string();
        Ok(name)
    }

    /// 获取最近一局的伤害排名
    pub async fn get_last_game_damage(&self) -> Result<Vec<PlayerDamage>, Box<dyn std::error::Error>> {
        // 获取当前召唤师 PUUID
        let summoner = self.api("/lol-summoner/v1/current-summoner").await?;
        let puuid = summoner["puuid"].as_str().ok_or("无法获取 PUUID")?;

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

        // 提取伤害数据
        let mut players: Vec<PlayerDamage> = participants
            .iter()
            .map(|p| PlayerDamage {
                champion_id: p["championId"].as_i64().unwrap_or(0),
                team_id: p["teamId"].as_i64().unwrap_or(0),
                damage: p["stats"]["totalDamageDealtToChampions"].as_i64().unwrap_or(0),
                kills: p["stats"]["kills"].as_i64().unwrap_or(0),
                deaths: p["stats"]["deaths"].as_i64().unwrap_or(0),
                assists: p["stats"]["assists"].as_i64().unwrap_or(0),
                is_lowest: false,
            })
            .collect();

        // 按伤害降序排序
        players.sort_by(|a, b| b.damage.cmp(&a.damage));

        // 标记最低伤害
        if let Some(last) = players.last_mut() {
            last.is_lowest = true;
        }

        Ok(players)
    }
}
