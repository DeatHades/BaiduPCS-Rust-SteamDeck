//! GameBox 安装流程 API

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::server::ApiResult;
use crate::AppState;

use crate::gamebox::{
    ExeFinder, Extractor, GameBoxError, GameBoxResult, GameBoxState, GameInfo, GameStatus,
    InstallProgress, InstallStep, SteamManager,
};
use crate::server::error::ApiError;

/// 开始安装任务
#[derive(Debug, Deserialize)]
pub struct StartInstallRequest {
    pub archive_path: String,
    pub install_dir: String,
    pub game_name: String,
    pub exe_path: Option<String>,
    pub proton_version: Option<String>,
    pub launch_options: Option<String>,
    pub delete_archive: bool,
}

/// 启动安装流程
pub async fn start_install(
    State(app_state): State<Arc<AppState>>,
    Json(req): Json<StartInstallRequest>,
) -> ApiResult<Json<InstallProgress>> {
    let task_id = uuid::Uuid::new_v4().to_string();
    let (progress_tx, mut progress_rx) = mpsc::channel::<InstallProgress>(100);

    // 创建安装任务
    let game = GameInfo {
        id: task_id.clone(),
        name: req.game_name.clone(),
        install_dir: req.install_dir.clone(),
        exe_path: req.exe_path.clone(),
        proton_version: req.proton_version.clone(),
        launch_options: req.launch_options.clone(),
        steam_app_id: None,
        status: GameStatus::Pending,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    // 初始化进度
    let _ = progress_tx
        .send(InstallProgress {
            task_id: task_id.clone(),
            step: InstallStep::Preparing,
            progress: 0.0,
            message: "准备安装...".to_string(),
            completed: false,
            failed: false,
            error: None,
        })
        .await;

    // 创建 GameBoxState 引用
    let gamebox_state = app_state.gamebox.clone();

    // 后台执行安装
    tokio::spawn(async move {
        let extractor = Extractor::new(gamebox_state.clone());
        let result = install_game(
            &extractor,
            &gamebox_state,
            &req,
            &task_id,
            &progress_tx,
            game,
        )
        .await;

        if let Err(e) = result {
            let _ = progress_tx
                .send(InstallProgress {
                    task_id: task_id.clone(),
                    step: InstallStep::Failed,
                    progress: 0.0,
                    message: format!("安装失败: {}", e),
                    completed: true,
                    failed: true,
                    error: Some(e.to_string()),
                })
                .await;
        }
    });

    // 返回初始进度
    if let Some(progress) = progress_rx.recv().await {
        Ok(Json(progress))
    } else {
        Err(ApiError::Internal(anyhow::anyhow!("无法获取安装进度")))
    }
}

/// 执行安装流程
async fn install_game(
    extractor: &Extractor,
    gamebox_state: &Arc<GameBoxState>,
    req: &StartInstallRequest,
    task_id: &str,
    progress_tx: &mpsc::Sender<InstallProgress>,
    mut game: GameInfo,
) -> GameBoxResult<()> {
    // 步骤 1: 解压
    let _ = progress_tx
        .send(InstallProgress {
            task_id: task_id.to_string(),
            step: InstallStep::Extracting,
            progress: 0.0,
            message: "开始解压...".to_string(),
            completed: false,
            failed: false,
            error: None,
        })
        .await;

    let extracted_dir = extractor
        .extract(
            &req.archive_path,
            &req.install_dir,
            |progress: f32, msg: &str| {
                let _ = progress_tx.try_send(InstallProgress {
                    task_id: task_id.to_string(),
                    step: InstallStep::Extracting,
                    progress,
                    message: msg.to_string(),
                    completed: false,
                    failed: false,
                    error: None,
                });
            },
        )
        .await?;

    // 更新状态
    game.status = GameStatus::Extracted;
    game.install_dir = extracted_dir;

    // 步骤 2: 查找主程序
    let _ = progress_tx
        .send(InstallProgress {
            task_id: task_id.to_string(),
            step: InstallStep::FindingExe,
            progress: 0.0,
            message: "查找游戏主程序...".to_string(),
            completed: false,
            failed: false,
            error: None,
        })
        .await;

    let exe_path = if let Some(ref exe) = req.exe_path {
        exe.clone()
    } else {
        match ExeFinder::find_main_executable(&game.install_dir, 6) {
            Ok(Some(found)) => {
                let _ = progress_tx
                    .send(InstallProgress {
                        task_id: task_id.to_string(),
                        step: InstallStep::FindingExe,
                        progress: 100.0,
                        message: format!("找到: {}", found.name),
                        completed: false,
                        failed: false,
                        error: None,
                    })
                    .await;
                found.path
            }
            Ok(None) => {
                return Err(GameBoxError::ExecutableNotFound);
            }
            Err(e) => {
                return Err(e);
            }
        }
    };

    game.exe_path = Some(exe_path.clone());
    game.status = GameStatus::Configuring;

    // 步骤 3: 添加到 Steam
    let _ = progress_tx
        .send(InstallProgress {
            task_id: task_id.to_string(),
            step: InstallStep::AddingToSteam,
            progress: 0.0,
            message: "添加到 Steam...".to_string(),
            completed: false,
            failed: false,
            error: None,
        })
        .await;

    let steam_manager = SteamManager::detect()?;
    let app_id = steam_manager.add_shortcut(&game)?;

    game.steam_app_id = Some(app_id);
    game.status = GameStatus::Installed;

    // 步骤 4: 完成
    let _ = progress_tx
        .send(InstallProgress {
            task_id: task_id.to_string(),
            step: InstallStep::Completed,
            progress: 100.0,
            message: format!("安装完成！Steam App ID: {}", app_id),
            completed: true,
            failed: false,
            error: None,
        })
        .await;

    // 删除压缩包
    if req.delete_archive {
        if let Err(e) = extractor.delete_archive(&req.archive_path).await {
            tracing::warn!("删除压缩包失败: {}", e);
        }
    }

    // 保存游戏信息
    gamebox_state.add_game(game);

    Ok(())
}

/// 获取安装进度
#[derive(Debug, Serialize)]
pub struct ProgressResponse {
    pub task_id: String,
    pub step: InstallStep,
    pub progress: f32,
    pub message: String,
    pub completed: bool,
    pub failed: bool,
    pub error: Option<String>,
}

pub async fn get_install_status(
    State(_app_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> ApiResult<Json<ProgressResponse>> {
    // TODO: 从安装任务列表中获取进度
    // 目前返回占位数据
    Ok(Json(ProgressResponse {
        task_id,
        step: InstallStep::Preparing,
        progress: 0.0,
        message: "安装任务已创建".to_string(),
        completed: false,
        failed: false,
        error: None,
    }))
}
