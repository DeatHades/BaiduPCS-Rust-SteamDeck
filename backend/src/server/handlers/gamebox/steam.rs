//! GameBox Steam 相关 API

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::server::handlers::ApiResult;
use crate::AppState;

use super::super::super::gamebox::{ExeFinder, ExecutableInfo, GameInfo, GameStatus, ProtonVersion, ShortcutInfo, SteamManager};

/// 查找可执行文件请求
#[derive(Debug, Deserialize)]
pub struct FindExecutablesRequest {
    pub directory: String,
    pub max_depth: Option<usize>,
}

/// 查找可执行文件
pub async fn find_executables(
    State(_app_state): State<Arc<AppState>>,
    Json(req): Json<FindExecutablesRequest>,
) -> ApiResult<Json<Vec<ExecutableInfo>>> {
    let max_depth = req.max_depth.unwrap_or(6);

    let executables = ExeFinder::find_executables(&req.directory, max_depth)?;

    Ok(Json(executables))
}

/// 获取 Steam 快捷方式列表
pub async fn list_shortcuts(
    State(_app_state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ShortcutInfo>>> {
    let steam_manager = SteamManager::detect()?;
    let shortcuts = steam_manager.list_shortcuts()?;

    Ok(Json(shortcuts))
}

/// 添加游戏到 Steam
#[derive(Debug, Deserialize)]
pub struct AddToSteamRequest {
    pub game_name: String,
    pub exe_path: String,
    pub proton_version: Option<String>,
    pub launch_options: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AddToSteamResponse {
    pub app_id: u64,
    pub success: bool,
}

/// 添加游戏到 Steam
pub async fn add_to_steam(
    State(_app_state): State<Arc<AppState>>,
    Json(req): Json<AddToSteamRequest>,
) -> ApiResult<Json<AddToSteamResponse>> {
    let steam_manager = SteamManager::detect()?;

    let game = GameInfo {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.game_name,
        install_dir: String::new(),
        exe_path: Some(req.exe_path),
        proton_version: req.proton_version,
        launch_options: req.launch_options,
        steam_app_id: None,
        status: GameStatus::Installed,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    let app_id = steam_manager.add_shortcut(&game)?;

    Ok(Json(AddToSteamResponse {
        app_id,
        success: true,
    }))
}

/// 从 Steam 移除游戏
pub async fn remove_from_steam(
    State(_app_state): State<Arc<AppState>>,
    Path(app_id): Path<u64>,
) -> ApiResult<Json<serde_json::Value>> {
    let steam_manager = SteamManager::detect()?;
    steam_manager.remove_shortcut(app_id)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "app_id": app_id
    })))
}

/// 获取可用的 Proton 版本
pub async fn list_proton_versions(
    State(_app_state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<ProtonVersion>>> {
    let steam_manager = match SteamManager::detect() {
        Ok(sm) => sm,
        Err(_) => {
            // Steam 未安装，返回默认列表
            return Ok(Json(vec![
                ProtonVersion {
                    name: "proton_experimental".to_string(),
                    display_name: "Proton 实验版".to_string(),
                    available: false,
                },
                ProtonVersion {
                    name: "proton_steam_deck".to_string(),
                    display_name: "Proton Steam Deck".to_string(),
                    available: false,
                },
                ProtonVersion {
                    name: "proton_8".to_string(),
                    display_name: "Proton 8".to_string(),
                    available: false,
                },
            ]));
        }
    };

    let versions = steam_manager.list_proton_versions();

    let proton_versions: Vec<ProtonVersion> = versions
        .into_iter()
        .map(|(name, display, available)| ProtonVersion {
            name,
            display_name: display,
            available,
        })
        .collect();

    Ok(Json(proton_versions))
}

/// 设置游戏的 Proton 版本
#[derive(Debug, Deserialize)]
pub struct SetProtonRequest {
    pub app_id: u64,
    pub proton_version: String,
}

pub async fn set_proton(
    State(_app_state): State<Arc<AppState>>,
    Json(req): Json<SetProtonRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let steam_manager = SteamManager::detect()?;
    steam_manager.set_proton(req.app_id, &req.proton_version)?;

    Ok(Json(serde_json::json!({
        "success": true,
        "app_id": req.app_id,
        "proton_version": req.proton_version
    })))
}
