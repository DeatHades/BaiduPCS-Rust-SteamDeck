//! GameBox 状态管理

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use tokio::sync::mpsc;

use super::types::{GameBoxConfig, GameInfo, InstallProgress, InstallStep};

/// 安装任务状态
pub struct InstallTask {
    /// 任务 ID
    pub task_id: String,
    /// 游戏信息
    pub game: GameInfo,
    /// 当前进度
    pub progress: InstallProgress,
    /// 进度更新通道
    pub progress_tx: mpsc::Sender<InstallProgress>,
    /// 取消标志
    pub cancelled: bool,
}

impl InstallTask {
    pub fn new(
        task_id: String,
        game: GameInfo,
        progress_tx: mpsc::Sender<InstallProgress>,
    ) -> Self {
        let progress = InstallProgress {
            task_id: task_id.clone(),
            step: InstallStep::Preparing,
            progress: 0.0,
            message: "准备中...".to_string(),
            completed: false,
            failed: false,
            error: None,
        };

        Self {
            task_id,
            game,
            progress,
            progress_tx,
            cancelled: false,
        }
    }

    /// 更新进度
    pub async fn update_progress(&mut self, step: InstallStep, progress: f32, message: &str) {
        self.progress.step = step;
        self.progress.progress = progress;
        self.progress.message = message.to_string();
        let _ = self.progress_tx.send(self.progress.clone()).await;
    }

    /// 标记失败
    pub async fn mark_failed(&mut self, error: &str) {
        self.progress.failed = true;
        self.progress.completed = true;
        self.progress.step = InstallStep::Failed;
        self.progress.error = Some(error.to_string());
        let _ = self.progress_tx.send(self.progress.clone()).await;
    }

    /// 标记完成
    pub async fn mark_completed(&mut self, message: &str) {
        self.progress.completed = true;
        self.progress.failed = false;
        self.progress.step = InstallStep::Completed;
        self.progress.progress = 100.0;
        self.progress.message = message.to_string();
        let _ = self.progress_tx.send(self.progress.clone()).await;
    }
}

/// GameBox 全局状态
pub struct GameBoxState {
    /// 配置
    pub config: RwLock<GameBoxConfig>,
    /// 已安装的游戏列表
    pub games: RwLock<Vec<GameInfo>>,
    /// 当前安装任务
    pub install_tasks: RwLock<HashMap<String, Arc<RwLock<InstallTask>>>>,
    /// 7z 可执行文件路径
    pub seven_zip_bin: RwLock<Option<PathBuf>>,
}

impl GameBoxState {
    /// 创建新的 GameBoxState
    pub fn new() -> Self {
        Self {
            config: RwLock::new(GameBoxConfig::default()),
            games: RwLock::new(Vec::new()),
            install_tasks: RwLock::new(HashMap::new()),
            seven_zip_bin: RwLock::new(None),
        }
    }

    /// 加载配置
    pub fn load_config(&self) -> GameBoxResult<()> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: GameBoxConfig = serde_json::from_str(&content)
                .map_err(|e| GameBoxError::ConfigLoadFailed(e.to_string()))?;
            *self.config.write() = config;
        }
        Ok(())
    }

    /// 保存配置
    pub fn save_config(&self) -> GameBoxResult<()> {
        let config_path = Self::config_path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let config = self.config.read().clone();
        let content = serde_json::to_string_pretty(&config)
            .map_err(|e| GameBoxError::ConfigSaveFailed(e.to_string()))?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// 获取配置路径
    fn config_path() -> GameBoxResult<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| GameBoxError::ConfigLoadFailed("无法获取配置目录".to_string()))?;
        Ok(config_dir.join("BaiduPCS-Rust").join("gamebox.json"))
    }

    /// 添加游戏
    pub fn add_game(&self, game: GameInfo) {
        self.games.write().push(game);
    }

    /// 获取游戏列表
    pub fn get_games(&self) -> Vec<GameInfo> {
        self.games.read().clone()
    }

    /// 根据 ID 获取游戏
    pub fn get_game(&self, id: &str) -> Option<GameInfo> {
        self.games.read().iter().find(|g| g.id == id).cloned()
    }

    /// 更新游戏
    pub fn update_game(&self, game: &GameInfo) {
        let mut games = self.games.write();
        if let Some(existing) = games.iter_mut().find(|g| g.id == game.id) {
            *existing = game.clone();
        }
    }

    /// 删除游戏
    pub fn remove_game(&self, id: &str) -> Option<GameInfo> {
        let mut games = self.games.write();
        if let Some(pos) = games.iter().position(|g| g.id == id) {
            Some(games.remove(pos))
        } else {
            None
        }
    }

    /// 添加安装任务
    pub fn add_install_task(&self, task: Arc<RwLock<InstallTask>>) {
        let task_id = task.read().task_id.clone();
        self.install_tasks.write().insert(task_id, task);
    }

    /// 获取安装任务
    pub fn get_install_task(&self, task_id: &str) -> Option<Arc<RwLock<InstallTask>>> {
        self.install_tasks.read().get(task_id).cloned()
    }

    /// 移除安装任务
    pub fn remove_install_task(&self, task_id: &str) {
        self.install_tasks.write().remove(task_id);
    }

    /// 设置 7z 路径
    pub fn set_seven_zip_bin(&self, path: Option<PathBuf>) {
        *self.seven_zip_bin.write() = path;
    }

    /// 获取 7z 路径
    pub fn get_seven_zip_bin(&self) -> Option<PathBuf> {
        self.seven_zip_bin.read().clone()
    }
}

impl Default for GameBoxState {
    fn default() -> Self {
        Self::new()
    }
}

use super::error::GameBoxResult;
