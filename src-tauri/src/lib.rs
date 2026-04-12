mod lcu;

use lcu::{GameResult, LcuConnection, MatchSummary};

#[tauri::command]
async fn check_connection() -> Result<String, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_current_summoner().await.map_err(|e| e.to_string())
}

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

/// 获取最近 N 局概要列表
#[tauri::command]
async fn get_match_list(count: Option<usize>) -> Result<Vec<MatchSummary>, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_match_list(count.unwrap_or(20)).await.map_err(|e| e.to_string())
}

/// 获取指定对局的红包局结果
#[tauri::command]
async fn get_game_result(game_id: i64) -> Result<GameResult, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_game_result(game_id).await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            check_connection,
            get_gameflow_phase,
            get_damage_ranking,
            get_match_list,
            get_game_result
        ])
        .setup(|app| {
            // 窗口定位到屏幕右下角
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(Some(monitor)) = window.current_monitor() {
                    let screen = monitor.size();
                    let scale = monitor.scale_factor();
                    let win_w = (380.0 * scale) as u32;
                    let win_h = (560.0 * scale) as u32;
                    let x = screen.width.saturating_sub(win_w).saturating_sub(16) as i32;
                    let y = screen.height.saturating_sub(win_h).saturating_sub(48) as i32;
                    let _ = window.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition::new(x, y),
                    ));
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
