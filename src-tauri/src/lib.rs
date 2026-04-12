mod lcu;

use lcu::{GameResult, LcuConnection, MatchSummary};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// 应用状态
pub struct AppState {
    /// 选英雄阶段的楼层映射 (championId → floor 1-5)
    pub champ_select_order: Mutex<HashMap<i64, usize>>,
    /// 本地存储路径
    pub history_path: Mutex<PathBuf>,
}

// ────── 本地存储 ──────

fn load_stored_results(path: &std::path::Path) -> HashMap<i64, GameResult> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_game_result(path: &std::path::Path, result: &GameResult) {
    let mut stored = load_stored_results(path);
    // 不要用近似数据覆盖精确数据
    if let Some(existing) = stored.get(&result.game_id) {
        if existing.has_accurate_floors && !result.has_accurate_floors {
            return;
        }
    }
    stored.insert(result.game_id, result.clone());
    if let Ok(json) = serde_json::to_string(&stored) {
        let _ = std::fs::write(path, json);
    }
}

// ────── Tauri 命令 ──────

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

/// 选英雄阶段抓取楼层顺序
#[tauri::command]
async fn capture_champ_select(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    let order = conn.get_champ_select_order().await.map_err(|e| e.to_string())?;
    if !order.is_empty() {
        *state.champ_select_order.lock().unwrap() = order;
    }
    Ok(())
}

/// 获取最近一局结果（使用选英雄阶段楼层 + 自动存储到本地）
#[tauri::command]
async fn get_damage_ranking(state: tauri::State<'_, AppState>) -> Result<GameResult, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    let order = state.champ_select_order.lock().unwrap().clone();
    let result = conn.get_last_game_result(&order).await.map_err(|e| e.to_string())?;

    // 存储到本地
    let path = state.history_path.lock().unwrap().clone();
    if !path.as_os_str().is_empty() {
        save_game_result(&path, &result);
    }

    Ok(result)
}

#[tauri::command]
async fn get_match_list(count: Option<usize>) -> Result<Vec<MatchSummary>, String> {
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_match_list(count.unwrap_or(20)).await.map_err(|e| e.to_string())
}

/// 获取指定对局结果（优先从本地存储读取）
#[tauri::command]
async fn get_game_result(game_id: i64, state: tauri::State<'_, AppState>) -> Result<GameResult, String> {
    // 先查本地存储
    let path = state.history_path.lock().unwrap().clone();
    if !path.as_os_str().is_empty() {
        let stored = load_stored_results(&path);
        if let Some(result) = stored.get(&game_id) {
            return Ok(result.clone());
        }
    }

    // 本地没有，从 API 获取（楼层为近似值）
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    conn.get_game_result(game_id, &HashMap::new()).await.map_err(|e| e.to_string())
}

/// 获取 Windows 工作区域
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
            history_path: Mutex::new(PathBuf::new()),
        })
        .invoke_handler(tauri::generate_handler![
            check_connection,
            get_gameflow_phase,
            capture_champ_select,
            get_damage_ranking,
            get_match_list,
            get_game_result
        ])
        .setup(|app| {
            use tauri::Manager;

            // 初始化本地存储路径
            if let Ok(data_dir) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(&data_dir);
                let history_path = data_dir.join("game_history.json");
                *app.state::<AppState>().history_path.lock().unwrap() = history_path;
            }

            // Windows: 窗口定位到右下角
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                let (_, _, work_right, work_bottom) = get_work_area();
                let win_w = 420;
                let win_h = 540;
                let x = work_right - win_w - 12;
                let y = work_bottom - win_h - 12;
                let _ = window.set_position(tauri::Position::Physical(
                    tauri::PhysicalPosition::new(x, y),
                ));
                let _ = window.set_size(tauri::Size::Physical(
                    tauri::PhysicalSize::new(win_w as u32, win_h as u32),
                ));
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("启动应用失败");
}
