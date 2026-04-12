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
    /// 缓存的 LCU 连接凭证 (port, token)，避免每次命令都执行 wmic
    pub lcu_credentials: Mutex<Option<(String, String)>>,
    /// 文件 I/O 锁
    pub file_lock: Mutex<()>,
}

// ────── LCU 连接（带缓存） ──────

async fn get_connection(state: &AppState) -> Result<LcuConnection, String> {
    // 先尝试用缓存的凭证
    let cached = state.lcu_credentials.lock().unwrap().clone();
    if let Some((port, token)) = cached {
        match LcuConnection::from_credentials(port.clone(), token.clone()).await {
            Ok(conn) => {
                // 验证连接是否还有效
                if conn.get_current_summoner().await.is_ok() {
                    return Ok(conn);
                }
            }
            Err(_) => {}
        }
        // 缓存失效，清除
        *state.lcu_credentials.lock().unwrap() = None;
    }

    // 重新检测
    let conn = LcuConnection::connect().await.map_err(|e| e.to_string())?;
    // 缓存凭证
    let (port, token) = conn.credentials();
    *state.lcu_credentials.lock().unwrap() = Some((port, token));
    Ok(conn)
}

// ────── 本地存储 ──────

fn load_stored_results(path: &std::path::Path) -> HashMap<i64, GameResult> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_game_result(state: &AppState, result: &GameResult) {
    let path = state.history_path.lock().unwrap().clone();
    if path.as_os_str().is_empty() {
        return;
    }
    // 文件锁保护
    let _lock = state.file_lock.lock().unwrap();
    let mut stored = load_stored_results(&path);
    if let Some(existing) = stored.get(&result.game_id) {
        if existing.has_accurate_floors && !result.has_accurate_floors {
            return;
        }
    }
    stored.insert(result.game_id, result.clone());
    if let Ok(json) = serde_json::to_string(&stored) {
        let _ = std::fs::write(&path, json);
    }
}

// ────── Tauri 命令 ──────

#[tauri::command]
async fn check_connection(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let conn = get_connection(&state).await?;
    conn.get_current_summoner().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_gameflow_phase(state: tauri::State<'_, AppState>) -> Result<String, String> {
    let conn = get_connection(&state).await?;
    conn.get_gameflow_phase().await.map_err(|e| e.to_string())
}

/// 选英雄阶段抓取楼层顺序
#[tauri::command]
async fn capture_champ_select(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let conn = get_connection(&state).await?;
    let order = conn.get_champ_select_order().await.map_err(|e| e.to_string())?;
    if !order.is_empty() {
        *state.champ_select_order.lock().unwrap() = order;
    }
    Ok(())
}

/// 获取最近一局结果（使用选英雄阶段楼层 + 自动存储到本地）
#[tauri::command]
async fn get_damage_ranking(state: tauri::State<'_, AppState>) -> Result<GameResult, String> {
    let conn = get_connection(&state).await?;
    let order = state.champ_select_order.lock().unwrap().clone();
    let result = conn.get_last_game_result(&order).await.map_err(|e| e.to_string())?;
    save_game_result(&state, &result);
    Ok(result)
}

#[tauri::command]
async fn get_match_list(state: tauri::State<'_, AppState>) -> Result<Vec<MatchSummary>, String> {
    let conn = get_connection(&state).await?;
    conn.get_match_list(20).await.map_err(|e| e.to_string())
}

/// 获取指定对局结果（优先从本地存储读取）
#[tauri::command]
async fn get_game_result(game_id: i64, state: tauri::State<'_, AppState>) -> Result<GameResult, String> {
    let path = state.history_path.lock().unwrap().clone();
    if !path.as_os_str().is_empty() {
        let _lock = state.file_lock.lock().unwrap();
        let stored = load_stored_results(&path);
        drop(_lock);
        if let Some(result) = stored.get(&game_id) {
            return Ok(result.clone());
        }
    }
    let conn = get_connection(&state).await?;
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
            lcu_credentials: Mutex::new(None),
            file_lock: Mutex::new(()),
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

            if let Ok(data_dir) = app.path().app_data_dir() {
                let _ = std::fs::create_dir_all(&data_dir);
                let history_path = data_dir.join("game_history.json");
                *app.state::<AppState>().history_path.lock().unwrap() = history_path;
            }

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
