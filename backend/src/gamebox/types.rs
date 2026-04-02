//! GameBox 模块数据类型定义

use serde::{Deserialize, Serialize};

/// 游戏信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameInfo {
    /// 游戏 ID（唯一标识）
    pub id: String,
    /// 游戏显示名称
    pub name: String,
    /// 游戏安装目录
    pub install_dir: String,
    /// 主程序路径
    pub exe_path: Option<String>,
    /// Proton 版本
    pub proton_version: Option<String>,
    /// 启动参数
    pub launch_options: Option<String>,
    /// Steam App ID（添加后有值）
    pub steam_app_id: Option<u64>,
    /// 游戏状态
    pub status: GameStatus,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 游戏状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    /// 待解压
    Pending,
    /// 解压中
    Extracting,
    /// 已解压（待配置）
    Extracted,
    /// 配置中
    Configuring,
    /// 已添加到 Steam
    Installed,
    /// 安装失败
    Failed,
}

impl Default for GameStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// 可执行文件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableInfo {
    /// 文件路径
    pub path: String,
    /// 文件名
    pub name: String,
    /// 扩展名
    pub extension: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 评分（越高越可能是主程序）
    pub score: i32,
    /// 目录深度
    pub depth: usize,
}

/// 压缩包信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveInfo {
    /// 文件路径
    pub path: String,
    /// 文件名
    pub name: String,
    /// 文件大小
    pub size: u64,
    /// 是否分卷压缩
    pub is_multipart: bool,
    /// 分卷数量
    pub part_count: Option<usize>,
}

/// 安装请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallRequest {
    /// 压缩包路径
    pub archive_path: String,
    /// 安装目录
    pub install_dir: String,
    /// 游戏名称
    pub game_name: String,
    /// 主程序路径（可选，如果为 None 则自动查找）
    pub exe_path: Option<String>,
    /// Proton 版本
    pub proton_version: Option<String>,
    /// 启动参数
    pub launch_options: Option<String>,
    /// 是否删除压缩包
    pub delete_archive: bool,
}

/// 安装进度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    /// 任务 ID
    pub task_id: String,
    /// 当前步骤
    pub step: InstallStep,
    /// 进度百分比 (0-100)
    pub progress: f32,
    /// 状态消息
    pub message: String,
    /// 是否完成
    pub completed: bool,
    /// 是否失败
    pub failed: bool,
    /// 错误信息
    pub error: Option<String>,
}

/// 安装步骤
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallStep {
    /// 准备中
    Preparing,
    /// 解压中
    Extracting,
    /// 查找主程序
    FindingExe,
    /// 添加到 Steam
    AddingToSteam,
    /// 配置 Proton
    ConfiguringProton,
    /// 完成
    Completed,
    /// 失败
    Failed,
}

/// Steam 快捷方式信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutInfo {
    /// App ID
    pub app_id: u64,
    /// 游戏名称
    pub app_name: String,
    /// 可执行文件路径
    pub exe_path: String,
    /// 启动目录
    pub start_dir: String,
    /// 启动参数
    pub launch_options: Option<String>,
    /// Proton 版本
    pub proton_version: Option<String>,
    /// 是否隐藏
    pub is_hidden: bool,
    /// 标签
    pub tags: Vec<String>,
}

/// Proton 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtonVersion {
    /// 版本名称（如 proton_experimental, proton_8, etc.）
    pub name: String,
    /// 显示名称
    pub display_name: String,
    /// 是否可用
    pub available: bool,
}

/// 目录扫描结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// 扫描目录
    pub directory: String,
    /// 找到的游戏
    pub games: Vec<GameInfo>,
    /// 找到的压缩包
    pub archives: Vec<ArchiveInfo>,
}

/// 配置信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameBoxConfig {
    /// 默认游戏安装目录
    pub default_install_dir: String,
    /// 默认 Proton 版本
    pub default_proton: String,
    /// 自动删除压缩包
    pub auto_delete_archive: bool,
    /// 默认启动参数
    pub default_launch_options: Option<String>,
    /// Steam 根目录（自动检测）
    pub steam_root: Option<String>,
}

impl Default for GameBoxConfig {
    fn default() -> Self {
        Self {
            default_install_dir: "~/Games".to_string(),
            default_proton: "proton_experimental".to_string(),
            auto_delete_archive: false,
            default_launch_options: None,
            steam_root: None,
        }
    }
}
