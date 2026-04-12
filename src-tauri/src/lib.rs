mod lcu;

use lcu::{GameResult, LcuConnection, MatchSummary};
use std::collections::HashMap;
use std::sync::Mutex;

/// 应用状态：存储选英雄阶段的楼层顺序
pub struct AppState {
    /// championId → floor (1-5)，选英雄阶段抓取
    pub champ_select_order: Mutex<HashMap<i64, usize>>,
}

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

/// 抓取选英雄阶段的楼层顺序
#[tauri::command]
async fn capture_champ_select(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    let order = conn.get_champ_select_order().await.map_err(|e| e.to_string())?;
    if !order.is_empty() {
        *state.champ_select_order.lock().unwrap() = order;
    }
    Ok(())
}

/// 获取最近一局的红包局结果（使用选英雄阶段的楼层数据）
#[tauri::command]
async fn get_damage_ranking(state: tauri::State<'_, AppState>) -> Result<GameResult, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    let order = state.champ_select_order.lock().unwrap().clone();
    conn.get_last_game_result(&order).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_match_list(count: Option<usize>) -> Result<Vec<MatchSummary>, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_match_list(count.unwrap_or(20)).await.map_err(|e| e.to_string())
}

/// 获取指定对局的红包局结果（历史对局无选英雄数据，用默认顺序）
#[tauri::command]
async fn get_game_result(game_id: i64) -> Result<GameResult, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_game_result(game_id, &HashMap::new()).await.map_err(|e| e.to_string())
}

/// 调试：查看原始参与者排序
#[tauri::command]
async fn debug_match_data() -> Result<String, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.debug_participant_order().await.map_err(|e| e.to_string())
}

/// 获取 Windows 工作区域（排除任务栏）
#[cfg(target_os = "windows")]
fn get_work_area() -> (i32, i32, i32, i32) {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;
    const SPI_GETWORKAREA: u32 = 0x0030;
    let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut rect as *mut _ as *mut _, 0); }
    (rect.left, rect.top, rect.right, rect.bottom)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            champ_select_order: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            check_connection,
            get_gameflow_phase,
            capture_champ_select,
            get_damage_ranking,
            get_match_list,
            get_game_result,
            debug_match_data
        ])
        .setup(|app| {
            #[cfg(target_os = "windows")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let (_, _, work_right, work_bottom) = get_work_area();
                    let win_w = 400;
                    let win_h = 480;
                    let x = work_right - win_w - 12;
                    let y = work_bottom - win_h - 12;
                    let _ = window.set_position(tauri::Position::Physical(
                        tauri::PhysicalPosition::new(x, y),
                    ));
                    let _ = window.set_size(tauri::Size::Physical(
                        tauri::PhysicalSize::new(win_w as u32, win_h as u32),
                    ));
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
