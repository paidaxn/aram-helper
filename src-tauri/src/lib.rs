mod lcu;

use lcu::{GameResult, LcuConnection};

/// 获取 LCU 连接状态，返回召唤师名称
#[tauri::command]
async fn check_connection() -> Result<String, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_current_summoner().await.map_err(|e| e.to_string())
}

/// 获取当前游戏流程阶段
#[tauri::command]
async fn get_gameflow_phase() -> Result<String, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_gameflow_phase().await.map_err(|e| e.to_string())
}

/// 获取最近一局的红包局结果
#[tauri::command]
async fn get_damage_ranking() -> Result<GameResult, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_last_game_result().await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            check_connection,
            get_gameflow_phase,
            get_damage_ranking
        ])
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
