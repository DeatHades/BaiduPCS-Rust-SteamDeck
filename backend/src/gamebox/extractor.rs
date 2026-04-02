//! 7z 解压引擎

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use super::error::{GameBoxError, GameBoxResult};
use super::types::ArchiveInfo;
use super::GameBoxState;

/// 支持的压缩包扩展名
const ARCHIVE_EXTENSIONS: &[&str] = &[
    "7z", "zip", "rar", "tar", "gz", "xz", "bz2", "tar.gz", "tar.xz", "tar.bz2", "tgz", "tbz2",
    "tar.zst", "zst",
];

/// 分卷压缩包模式
const MULTIPART_PATTERNS: &[&str] = &[
    ".7z.001", ".7z.002", ".7z.003", ".7z.004", ".7z.005", ".7z.006", ".7z.007", ".7z.008",
    ".7z.009", ".7z.010", ".part1.rar", ".part2.rar", ".r00", ".r01", ".r02",
];

/// 7z 二进制文件候选路径
const SEVEN_ZIP_CANDIDATES: &[&str] = &[
    // 内置 7zz (Freedeck 插件自带)
    "defaults/7z/linux-x86_64/7zz",
    "defaults/7z/linux-x64/7zz",
    "defaults/7z/7zz",
    "defaults/7zz",
    // 系统命令
    "/usr/bin/7zz",
    "/usr/bin/7zr",
    "/usr/bin/7z",
    "/usr/bin/p7zip",
];

/// 解压引擎
pub struct Extractor {
    state: Arc<GameBoxState>,
}

impl Extractor {
    /// 创建新的解压引擎
    pub fn new(state: Arc<GameBoxState>) -> Self {
        Self { state }
    }

    /// 查找 7z 可执行文件
    pub async fn find_seven_zip(&self) -> GameBoxResult<PathBuf> {
        // 1. 优先使用配置中的路径
        if let Some(path) = self.state.get_seven_zip_bin() {
            if path.exists() {
                return Ok(path);
            }
        }

        // 2. 尝试环境变量
        if let Ok(path) = std::env::var("FREEDECK_7Z_BIN") {
            let path = PathBuf::from(path);
            if path.exists() {
                self.state.set_seven_zip_bin(Some(path.clone()));
                return Ok(path);
            }
        }

        // 3. 查找内置 7zz
        for candidate in SEVEN_ZIP_CANDIDATES {
            let path = PathBuf::from(candidate);
            if path.exists() {
                // 确保可执行权限
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(mut perms) = std::fs::metadata(&path).map(|m| m.permissions()) {
                        let mode = perms.mode();
                        if mode & 0o111 == 0 {
                            perms.set_mode(mode | 0o111);
                            std::fs::set_permissions(&path, perms).ok();
                        }
                    }
                }
                self.state.set_seven_zip_bin(Some(path.clone()));
                return Ok(path);
            }
        }

        // 4. 尝试系统命令
        for cmd in &["7zz", "7zr", "7z"] {
            if Self::command_exists(cmd) {
                let path = PathBuf::from(cmd);
                self.state.set_seven_zip_bin(Some(path.clone()));
                return Ok(path);
            }
        }

        Err(GameBoxError::ExtractorNotFound)
    }

    /// 检查命令是否存在
    fn command_exists(cmd: &str) -> bool {
        std::process::Command::new("which")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// 扫描目录中的压缩包
    pub fn scan_archives(&self, directory: &str) -> GameBoxResult<Vec<ArchiveInfo>> {
        let dir = PathBuf::from(directory);
        if !dir.exists() {
            return Err(GameBoxError::DirectoryNotFound(directory.to_string()));
        }

        let mut archives = Vec::new();

        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            // 跳过隐藏文件和分卷文件（只收集 .7z.001 或 .part1.rar）
            if filename.starts_with('.') {
                continue;
            }

            // 检查是否是主分卷文件
            let is_main_volume = Self::is_main_volume_file(filename);
            if !Self::is_archive_file(filename) && !is_main_volume {
                continue;
            }

            let metadata = entry.metadata()?;
            let size = metadata.len();

            if is_main_volume {
                // 统计分卷数量
                let part_count = Self::count_volume_parts(&dir, filename);

                archives.push(ArchiveInfo {
                    path: path.to_string_lossy().to_string(),
                    name: filename.to_string(),
                    size,
                    is_multipart: true,
                    part_count: Some(part_count),
                });
            } else {
                archives.push(ArchiveInfo {
                    path: path.to_string_lossy().to_string(),
                    name: filename.to_string(),
                    size,
                    is_multipart: false,
                    part_count: None,
                });
            }
        }

        // 按文件名排序
        archives.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(archives)
    }

    /// 检查是否是压缩包文件
    fn is_archive_file(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        for ext in ARCHIVE_EXTENSIONS {
            if lower.ends_with(&format!(".{}", ext)) {
                return true;
            }
        }
        false
    }

    /// 检查是否是分卷压缩包的主文件
    fn is_main_volume_file(filename: &str) -> bool {
        let lower = filename.to_lowercase();
        lower.ends_with(".7z.001") || lower.ends_with(".7z.01") || lower.ends_with(".part1.rar")
    }

    /// 统计分卷数量
    fn count_volume_parts(dir: &Path, main_file: &str) -> usize {
        let lower = main_file.to_lowercase();
        let base_name;

        if lower.ends_with(".7z.001") {
            base_name = &main_file[..main_file.len() - 8]; // 去掉 ".7z.001"
        } else if lower.ends_with(".7z.01") {
            base_name = &main_file[..main_file.len() - 7]; // 去掉 ".7z.01"
        } else if lower.ends_with(".part1.rar") {
            base_name = &main_file[..main_file.len() - 9]; // 去掉 ".part1.rar"
        } else {
            return 1;
        }

        let upper = base_name.to_uppercase();
        let lower_base = base_name.to_lowercase();

        // 统计同系列分卷
        let mut count = 1;
        loop {
            count += 1;
            let test_name = format!("{}00{}", base_name, count);
            let test_name_upper = format!("{}00{}", upper, count);
            if !dir.join(&test_name).exists() && !dir.join(&test_name_upper).exists() {
                break;
            }
        }

        // 也检查 .7z.002, .7z.003 等
        for i in 2..=999 {
            let suffix = format!("{:03}", i);
            let suffix2 = format!("{:02}", i);
            let test = format!("{}{}.{}", base_name, suffix, "7z");
            let test_upper = format!("{}{}.{}", upper, suffix, "7z");
            if !dir.join(&test).exists() && !dir.join(&test_upper).exists() {
                break;
            }
        }

        count
    }

    /// 解压压缩包
    pub async fn extract(
        &self,
        archive_path: &str,
        output_dir: &str,
        progress_callback: impl Fn(f32, &str),
    ) -> GameBoxResult<String> {
        let seven_zip = self.find_seven_zip().await?;

        let archive = PathBuf::from(archive_path);
        let output = PathBuf::from(output_dir);

        if !archive.exists() {
            return Err(GameBoxError::FileNotFound(archive_path.to_string()));
        }

        // 创建输出目录
        tokio::fs::create_dir_all(&output).await?;

        // 检测分卷并收集所有分卷
        let mut args = Vec::new();
        let filename = archive.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        if Self::is_main_volume_file(filename) {
            // 分卷压缩，收集所有分卷
            if let Some(parent) = archive.parent() {
                let parts = Self::collect_volume_parts(parent, filename);
                progress_callback(0.0, &format!("找到 {} 个分卷...", parts.len()));
                for part in &parts {
                    args.push(part.clone());
                }
            }
        } else {
            args.push(archive.to_string_lossy().to_string());
        }

        // 构建解压命令
        let mut cmd = Command::new(&seven_zip);
        cmd.arg("x")                        // 完整解压
           .arg("-y")                       // 自动确认
           .arg("-o".to_string() + output_dir) // 输出目录
           .arg("-bsp1")                    // 输出进度
           .args(&args);

        progress_callback(0.0, "开始解压...");

        // 执行命令
        let mut child = cmd.stdout(Stdio::piped())
                           .stderr(Stdio::piped())
                           .spawn()
                           .map_err(|e| GameBoxError::ExtractionFailed(e.to_string()))?;

        let stdout = child.stdout.take()
            .ok_or_else(|| GameBoxError::ExtractionFailed("无法捕获输出".to_string()))?;

        let mut reader = BufReader::new(stdout).lines();
        let mut last_progress = 0.0;

        while let Ok(Some(line)) = reader.next_line().await {
            // 解析 7z 进度 (格式: "25%" 或 "25% - filename")
            if let Some(progress) = Self::parse_progress(&line) {
                if progress > last_progress {
                    last_progress = progress;
                    progress_callback(progress, &line);
                }
            }
        }

        let status = child.wait().await
            .map_err(|e| GameBoxError::ExtractionFailed(e.to_string()))?;

        if !status.success() {
            return Err(GameBoxError::ExtractionFailed(format!(
                "解压命令返回非零状态: {}",
                status
            )));
        }

        progress_callback(100.0, "解压完成");

        // 返回解压内容目录
        Ok(output.to_string_lossy().to_string())
    }

    /// 解析 7z 进度
    fn parse_progress(line: &str) -> Option<f32> {
        // 匹配 "25%" 或 " 25%"
        let trimmed = line.trim();
        if trimmed.ends_with('%') {
            if let Ok(p) = trimmed[..trimmed.len()-1].trim().parse::<f32>() {
                return Some(p.clamp(0.0, 100.0));
            }
        }
        None
    }

    /// 收集分卷文件
    fn collect_volume_parts(dir: &Path, main_file: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let lower = main_file.to_lowercase();

        let (base_name, extension): (&str, &str) = if lower.ends_with(".7z.001") {
            (&main_file[..main_file.len() - 8], "7z")
        } else if lower.ends_with(".7z.01") {
            (&main_file[..main_file.len() - 7], "7z")
        } else if lower.ends_with(".part1.rar") {
            (&main_file[..main_file.len() - 9], "rar")
        } else {
            return vec![main_file.to_string()];
        };

        // 收集所有分卷
        let upper_base = base_name.to_uppercase();
        let lower_base = base_name.to_lowercase();

        for i in 1..=999 {
            let part_name = if i < 10 {
                format!("{}{:03}.{}", lower_base, i, extension)
            } else if i < 100 {
                format!("{}{:03}.{}", lower_base, i, extension)
            } else {
                format!("{}{:03}.{}", lower_base, i, extension)
            };

            let part_path = dir.join(&part_name);
            if part_path.exists() {
                parts.push(part_path.to_string_lossy().to_string());
            } else {
                // 尝试大写
                let upper_part = format!("{}{:03}.{}", upper_base, i, extension);
                let upper_path = dir.join(&upper_part);
                if upper_path.exists() {
                    parts.push(upper_path.to_string_lossy().to_string());
                }
            }
        }

        // 按文件名排序
        parts.sort();
        parts
    }

    /// 删除压缩包
    pub async fn delete_archive(&self, archive_path: &str) -> GameBoxResult<()> {
        let path = PathBuf::from(archive_path);
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
        }

        // 同时删除同名分卷
        if let Some(parent) = path.parent() {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                let parts = Self::collect_volume_parts(parent, filename);
                for part in parts {
                    let part_path = PathBuf::from(&part);
                    if part_path.exists() && part_path != path {
                        tokio::fs::remove_file(&part_path).await.ok();
                    }
                }
            }
        }

        Ok(())
    }
}
