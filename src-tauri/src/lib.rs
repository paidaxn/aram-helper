mod lcu;

use lcu::{LcuConnection, PlayerDamage};

/// 获取 LCU 连接状态
#[tauri::command]
async fn check_connection() -> Result<String, String> {
    match LcuConnection::connect().await {
        Ok(conn) => {
            let summoner = conn.get_current_summoner().await.map_err(|e| e.to_string())?;
            Ok(summoner)
        }
        Err(e) => Err(e.to_string()),
    }
}

/// 获取最近一局的伤害排名
#[tauri::command]
async fn get_damage_ranking() -> Result<Vec<PlayerDamage>, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_last_game_damage().await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![check_connection, get_damage_ranking])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
