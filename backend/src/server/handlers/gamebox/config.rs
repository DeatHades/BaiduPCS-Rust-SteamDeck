//! GameBox 配置管理 API

use std::sync::Arc;

use axum::{
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::server::ApiResult;
use crate::AppState;

use crate::gamebox::{GameBoxConfig, GameInfo, GameBoxState};

/// 获取配置
pub async fn get_config(
    State(app_state): State<Arc<AppState>>,
) -> ApiResult<Json<GameBoxConfig>> {
    let config = app_state.gamebox.config.read().clone();
    Ok(Json(config))
}

/// 更新配置
#[derive(Debug, Deserialize)]
pub struct UpdateConfigRequest {
    pub default_install_dir: Option<String>,
    pub default_proton: Option<String>,
    pub auto_delete_archive: Option<bool>,
    pub default_launch_options: Option<String>,
}

pub async fn update_config(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<UpdateConfigRequest>,
) -> ApiResult<Json<GameBoxConfig>> {
    let mut config = app_state.gamebox.config.write();

    if let Some(dir) = req.default_install_dir {
        config.default_install_dir = dir;
    }
    if let Some(proton) = req.default_proton {
        config.default_proton = proton;
    }
    if let Some(auto_delete) = req.auto_delete_archive {
        config.auto_delete_archive = auto_delete;
    }
    if let Some(launch_opts) = req.default_launch_options {
        config.default_launch_options = Some(launch_opts);
    }

    let updated_config = config.clone();
    drop(config);

    // 保存配置
    app_state.gamebox.save_config().ok();

    Ok(Json(updated_config))
}

/// 获取游戏列表
pub async fn list_games(
    State(app_state): State<Arc<AppState>>,
) -> ApiResult<Json<Vec<GameInfo>>> {
    let games = app_state.gamebox.get_games();
    Ok(Json(games))
}

/// 获取单个游戏信息
#[derive(Debug, Serialize)]
pub struct GameInfoResponse {
    pub game: Option<GameInfo>,
    pub found: bool,
}

pub async fn get_game(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(game_id): axum::extract::Path<String>,
) -> ApiResult<Json<GameInfoResponse>> {
    let game = app_state.gamebox.get_game(&game_id);

    Ok(Json(GameInfoResponse {
        found: game.is_some(),
        game,
    }))
}

/// 删除游戏记录
pub async fn delete_game(
    State(app_state): State<Arc<AppState>>,
    axum::extract::Path(game_id): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let removed = app_state.gamebox.remove_game(&game_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "removed": removed.is_some(),
        "game_id": game_id
    })))
}
