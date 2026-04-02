//! GameBox 压缩包操作 API

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::server::ApiResult;
use crate::AppState;

use crate::gamebox::{ArchiveInfo, Extractor, GameBoxError};

/// 扫描目录请求
#[derive(Debug, Deserialize)]
pub struct ScanDirectoryRequest {
    pub directory: String,
}

/// 扫描目录结果
#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub directory: String,
    pub archives: Vec<ArchiveInfo>,
}

/// 扫描目录中的压缩包
pub async fn scan_directory(
    State(app_state): State<AppState>,
    Json(req): Json<ScanDirectoryRequest>,
) -> ApiResult<Json<ScanResult>> {
    let extractor = Extractor::new(app_state.gamebox.clone());

    let archives = extractor
        .scan_archives(&req.directory)
        .map_err(GameBoxError::from)?;

    Ok(Json(ScanResult {
        directory: req.directory,
        archives,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ScanDirectoryQuery {
    pub directory: Option<String>,
}

/// 获取目录下的压缩包列表
pub async fn list_archives(
    State(app_state): State<AppState>,
    Query(params): Query<ScanDirectoryQuery>,
) -> ApiResult<Json<Vec<ArchiveInfo>>> {
    let extractor = Extractor::new(app_state.gamebox.clone());
    let directory = params.directory.unwrap_or_else(|| "~/Downloads".to_string());

    let archives = extractor
        .scan_archives(&directory)
        .map_err(GameBoxError::from)?;

    Ok(Json(archives))
}
