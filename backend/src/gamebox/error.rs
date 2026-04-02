//! GameBox 模块错误类型定义

use thiserror::Error;

/// GameBox 模块错误类型
#[derive(Error, Debug)]
pub enum GameBoxError {
    /// Steam 未找到
    #[error("Steam 未找到，请确保 Steam 已安装")]
    SteamNotFound,

    /// Steam 用户未找到
    #[error("Steam 用户未找到，请登录 Steam")]
    SteamUserNotFound,

    /// Steam 快捷方式文件不可访问
    #[error("Steam 快捷方式文件不可访问: {0}")]
    ShortcutsNotAccessible(String),

    /// 解压工具未找到
    #[error("解压工具未找到，请安装 7z (7zz 或 p7zip)")]
    ExtractorNotFound,

    /// 解压失败
    #[error("解压失败: {0}")]
    ExtractionFailed(String),

    /// 文件不存在
    #[error("文件不存在: {0}")]
    FileNotFound(String),

    /// 目录不存在
    #[error("目录不存在: {0}")]
    DirectoryNotFound(String),

    /// 游戏主程序未找到
    #[error("未找到游戏主程序，请手动指定")]
    ExecutableNotFound,

    /// 添加 Steam 快捷方式失败
    #[error("添加 Steam 快捷方式失败: {0}")]
    AddShortcutFailed(String),

    /// 设置 Proton 失败
    #[error("设置 Proton 失败: {0}")]
    SetProtonFailed(String),

    /// 读取 VDF 文件失败
    #[error("读取 VDF 文件失败: {0}")]
    VdfReadFailed(String),

    /// 写入 VDF 文件失败
    #[error("写入 VDF 文件失败: {0}")]
    VdfWriteFailed(String),

    /// 配置保存失败
    #[error("配置保存失败: {0}")]
    ConfigSaveFailed(String),

    /// 配置加载失败
    #[error("配置加载失败: {0}")]
    ConfigLoadFailed(String),

    /// 安装任务不存在
    #[error("安装任务不存在: {0}")]
    TaskNotFound(String),

    /// 安装正在进行中
    #[error("安装正在进行中: {0}")]
    TaskInProgress(String),

    /// IO 错误
    #[error("IO 错误: {0}")]
    IoError(String),

    /// JSON 序列化错误
    #[error("JSON 序列化错误: {0}")]
    JsonError(String),
}

impl From<std::io::Error> for GameBoxError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for GameBoxError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err.to_string())
    }
}

/// GameBox 结果类型
pub type GameBoxResult<T> = Result<T, GameBoxError>;
