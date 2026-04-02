//! 游戏主程序查找器

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::error::GameBoxResult;
use super::types::ExecutableInfo;

/// 可执行文件扩展名及其优先级
const EXECUTABLE_EXTENSIONS: &[(&str, i32)] = &[
    // Windows 可执行文件
    (".exe", 100),
    // Windows 脚本
    (".bat", 80),
    (".cmd", 80),
    // Linux 可执行文件
    (".sh", 60),
    (".x86_64", 50),
    (".x86", 50),
    (".amd64", 50),
    // AppImage
    (".appimage", 40),
    // 其他
    ("", 10),  // 无扩展名的可执行文件
];

/// 常见的游戏主程序名称（不区分大小写匹配，优先级加成）
const COMMON_GAME_EXECUTABLES: &[(&str, i32)] = &[
    // 通用
    ("game", 50),
    ("launcher", 40),
    ("play", 30),
    ("start", 20),
    ("startup", 20),
    // 常见游戏引擎
    ("ue4", 60),
    ("ue5", 60),
    ("unreal", 60),
    ("unity", 60),
    ("fna", 40),
    ("xna", 40),
    ("monogame", 40),
    // 模拟器
    ("emulator", 30),
    ("retroarch", 40),
    ("dolphin", 40),
    ("cemu", 40),
    ("yuzu", 40),
    ("ryujinx", 40),
    // 特定游戏
    ("eldenring", 80),
    ("elden_ring", 80),
    ("gta", 80),
    ("gtav", 80),
    ("reddeadredemption", 80),
    ("rdr2", 80),
    ("cyberpunk", 80),
    ("witcher", 80),
];

/// 游戏主程序查找器
pub struct ExeFinder;

impl ExeFinder {
    /// 查找目录中的所有可执行文件
    pub fn find_executables(root_dir: &str, max_depth: usize) -> GameBoxResult<Vec<ExecutableInfo>> {
        let root = PathBuf::from(root_dir);
        if !root.exists() {
            return Ok(Vec::new());
        }

        let mut executables = Vec::new();

        for entry in WalkDir::new(&root)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // 跳过目录
            if !path.is_file() {
                continue;
            }

            // 获取文件名和扩展名
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            // 跳过隐藏文件
            if filename.starts_with('.') {
                continue;
            }

            // 检查是否是可执行文件
            let extension = path.extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()))
                .unwrap_or_default();

            let ext_score = Self::get_extension_score(&extension);
            let name_score = Self::get_name_score(filename);
            let depth_score = Self::get_depth_score(path, &root);

            // 总评分
            let total_score = ext_score + name_score + depth_score;

            // 排除明显不是游戏的文件
            if Self::is_likely_game_file(filename) {
                let metadata = entry.metadata().ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

                executables.push(ExecutableInfo {
                    path: path.to_string_lossy().to_string(),
                    name: filename.to_string(),
                    extension: extension.clone(),
                    size,
                    score: total_score,
                    depth: Self::get_depth(path, &root),
                });
            }
        }

        // 按评分排序（降序）
        executables.sort_by(|a, b| b.score.cmp(&a.score));

        Ok(executables)
    }

    /// 查找最可能的主程序
    pub fn find_main_executable(root_dir: &str, max_depth: usize) -> GameBoxResult<Option<ExecutableInfo>> {
        let executables = Self::find_executables(root_dir, max_depth)?;

        // 返回评分最高的
        Ok(executables.into_iter().next())
    }

    /// 获取扩展名评分
    fn get_extension_score(extension: &str) -> i32 {
        for (ext, score) in EXECUTABLE_EXTENSIONS {
            if *ext == extension || (extension.is_empty() && *ext == "") {
                return *score;
            }
        }
        0
    }

    /// 获取文件名评分
    fn get_name_score(filename: &str) -> i32 {
        let lower = filename.to_lowercase();
        let mut score = 0;

        for (pattern, bonus) in COMMON_GAME_EXECUTABLES {
            if lower.contains(pattern) {
                score += *bonus;
            }
        }

        // 惩罚含有明显非游戏标识的
        let exclusions = ["uninstall", "setup", "install", "config", "crash", "helper", "updater", "redist"];
        for excl in exclusions {
            if lower.contains(excl) {
                score -= 30;
            }
        }

        // 惩罚路径中含有 common、redist、vc 等的
        // （由 depth_score 处理）

        score.max(0)
    }

    /// 获取深度评分（越浅越可能是主程序）
    fn get_depth_score(path: &Path, root: &Path) -> i32 {
        let depth = Self::get_depth(path, root);

        // 根目录 +10，每深入一层 -5
        match depth {
            0 => 20,
            1 => 15,
            2 => 10,
            3 => 5,
            _ => 0,
        }
    }

    /// 获取路径深度
    fn get_depth(path: &Path, root: &Path) -> usize {
        let path_components = path.components().count();
        let root_components = root.components().count();

        if path_components > root_components {
            path_components - root_components
        } else {
            0
        }
    }

    /// 检查是否是可能的游戏文件
    fn is_likely_game_file(filename: &str) -> bool {
        let lower = filename.to_lowercase();

        // 检查扩展名
        let has_executable_ext = EXECUTABLE_EXTENSIONS.iter().any(|(ext, _)| {
            if ext.is_empty() {
                // 无扩展名，检查是否是可执行文件
                false
            } else {
                lower.ends_with(ext)
            }
        });

        if !has_executable_ext {
            return false;
        }

        // 排除明显的非游戏文件
        let exclusions = [
            "unins",          // 卸载程序
            "uninstall",      // 卸载程序
            "crashhandler",   // 崩溃处理器
            "dxsetup",        // DirectX 安装
            "vcredist",       // VC++ 运行库
            "directx",        // DirectX
            " redist",        // 运行库
        ];

        for excl in exclusions {
            if lower.contains(excl) {
                return false;
            }
        }

        true
    }

    /// 查找目录下已安装的游戏（通过已配置的 shortcuts）
    pub fn find_installed_games(games_base_dir: &str) -> GameBoxResult<Vec<(PathBuf, String)>> {
        let base = PathBuf::from(games_base_dir);
        if !base.exists() {
            return Ok(Vec::new());
        }

        let mut games = Vec::new();

        for entry in std::fs::read_dir(&base)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            if name.is_empty() || name.starts_with('.') {
                continue;
            }

            games.push((path, name));
        }

        games.sort_by(|a, b| a.1.cmp(&b.1));

        Ok(games)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_scoring() {
        assert!(ExeFinder::get_extension_score(".exe") > ExeFinder::get_extension_score(".sh"));
        assert!(ExeFinder::get_extension_score(".bat") > ExeFinder::get_extension_score(".appimage"));
    }
}
