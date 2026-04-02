//! Steam shortcuts 和 Proton 配置管理

use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write as IoWrite, Seek, SeekFrom};
use std::path::PathBuf;

use super::error::{GameBoxError, GameBoxResult};
use super::types::{GameInfo, GameStatus, ShortcutInfo};

/// Steam 根目录候选
const STEAM_ROOT_CANDIDATES: &[&str] = &[
    ".steam/steam",
    ".steam/root",
    ".local/share/Steam",
    "/usr/share/steam",
    "/home/deck/.steam/steam",
];

/// Proton 版本
const PROTON_VERSIONS: &[(&str, &str)] = &[
    ("proton_experimental", "Proton 实验版"),
    ("proton_steam_deck", "Proton Steam Deck"),
    ("proton_8", "Proton 8"),
    ("proton_7", "Proton 7"),
    ("proton_6", "Proton 6"),
];

/// Steam 管理器
pub struct SteamManager {
    steam_root: PathBuf,
    user_id: String,
    shortcuts_path: PathBuf,
    config_vdf_path: PathBuf,
}

impl SteamManager {
    /// 检测 Steam 安装
    pub fn detect() -> GameBoxResult<Self> {
        let steam_root = Self::find_steam_root()?;

        let user_id = Self::detect_active_user(&steam_root)?;

        let shortcuts_path = steam_root
            .join("userdata")
            .join(&user_id)
            .join("config")
            .join("shortcuts.vdf");

        let config_vdf_path = steam_root.join("config").join("config.vdf");

        Ok(Self {
            steam_root,
            user_id,
            shortcuts_path,
            config_vdf_path,
        })
    }

    /// 查找 Steam 根目录
    fn find_steam_root() -> GameBoxResult<PathBuf> {
        let home = std::env::var("HOME")
            .map_err(|_| GameBoxError::SteamNotFound)?;

        for candidate in STEAM_ROOT_CANDIDATES {
            let path = PathBuf::from(&home).join(candidate);
            if path.exists() {
                let steamapps = path.join("steamapps");
                if steamapps.exists() {
                    return Ok(path);
                }
            }
        }

        // 尝试 Steam 环境变量
        if let Ok(steam_root) = std::env::var("STEAMROOT") {
            let path = PathBuf::from(steam_root);
            if path.join("steamapps").exists() {
                return Ok(path);
            }
        }

        Err(GameBoxError::SteamNotFound)
    }

    /// 检测当前登录的用户
    fn detect_active_user(steam_root: &PathBuf) -> GameBoxResult<String> {
        let userdata_root = steam_root.join("userdata");

        if !userdata_root.exists() {
            return Err(GameBoxError::SteamUserNotFound);
        }

        // 尝试从 loginusers.vdf 获取最近登录用户
        let loginusers_path = steam_root.join("config").join("loginusers.vdf");
        if let Ok(user_id) = Self::parse_loginusers(&loginusers_path) {
            if user_id.is_some() {
                return Ok(user_id.unwrap());
            }
        }

        // 回退：选择最新修改的用户目录（排除 user 0）
        let mut candidates: Vec<(String, std::time::SystemTime)> = Vec::new();

        if let Ok(entries) = fs::read_dir(&userdata_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                if !name.chars().all(|c| c.is_ascii_digit()) || name == "0" {
                    continue;
                }

                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        candidates.push((name.to_string(), modified));
                    }
                }
            }
        }

        if candidates.is_empty() {
            return Err(GameBoxError::SteamUserNotFound);
        }

        // 按修改时间降序排序
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(candidates[0].0.clone())
    }

    /// 解析 loginusers.vdf 获取最近登录用户
    fn parse_loginusers(path: &PathBuf) -> GameBoxResult<Option<String>> {
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path)?;

        // 简单的正则匹配找 MostRecent=1 的用户
        let mut most_recent: Option<String> = None;
        let mut current_id: Option<String> = None;
        let mut current_recent = false;

        for line in content.lines() {
            let line = line.trim();

            // 匹配 "76561198000000000" { ... }
            if line.starts_with('"') && !line.contains('{') {
                if let Some(end) = line.find("\"") {
                    let id = &line[1..end];
                    if id.chars().all(|c| c.is_ascii_digit()) {
                        current_id = Some(id.to_string());
                        current_recent = false;
                    }
                }
            }

            // 匹配 "MostRecent" "1"
            if line.contains("MostRecent") && line.contains('"') && line.contains("1") {
                if let Some(ref id) = current_id {
                    most_recent = Some(id.clone());
                    break;
                }
            }
        }

        Ok(most_recent)
    }

    /// 获取 shortcuts.vdf 路径
    pub fn shortcuts_path(&self) -> &PathBuf {
        &self.shortcuts_path
    }

    /// 获取 Steam 根目录
    pub fn steam_root(&self) -> &PathBuf {
        &self.steam_root
    }

    /// 获取用户 ID
    pub fn user_id(&self) -> &str {
        &self.user_id
    }

    /// 获取所有 Proton 版本
    pub fn list_proton_versions(&self) -> Vec<(String, String, bool)> {
        let compat_dir = self.steam_root.join("steamapps").join("common");

        PROTON_VERSIONS
            .iter()
            .map(|(name, display)| {
                let available = compat_dir.join(name).exists();
                (name.to_string(), display.to_string(), available)
            })
            .collect()
    }

    /// 列出所有快捷方式
    pub fn list_shortcuts(&self) -> GameBoxResult<Vec<ShortcutInfo>> {
        if !self.shortcuts_path.exists() {
            return Ok(Vec::new());
        }

        let data = fs::read(&self.shortcuts_path)?;

        // VDF 是二进制格式，简单解析
        // shortcuts.vdf 格式: "shortcuts" { "0" { "appid" ... "AppName" ... } }
        Self::parse_shortcuts_vdf(&data)
    }

    /// 解析 shortcuts.vdf 二进制格式
    fn parse_shortcuts_vdf(data: &[u8]) -> GameBoxResult<Vec<ShortcutInfo>> {
        // 简化实现：查找 AppName 和 exe 字段
        let mut shortcuts = Vec::new();
        let content = String::from_utf8_lossy(data);

        // 简单的字符串匹配解析
        // 注意：这是一个简化实现，实际应该使用专门的 VDF 解析库
        let mut current: Option<ShortcutInfo> = None;
        let mut in_app = false;

        for line in content.lines() {
            let line = line.trim();

            // 检测新的应用块开始
            if line.ends_with('{') && !line.contains("shortcuts") {
                if let Some(idx) = line.find("\"") {
                    if let Some(end) = line[idx..].find("\"") {
                        let key = &line[idx+1..idx+end];
                        if key.chars().all(|c| c.is_ascii_digit()) {
                            current = Some(ShortcutInfo {
                                app_id: key.parse().unwrap_or(0),
                                app_name: String::new(),
                                exe_path: String::new(),
                                start_dir: String::new(),
                                launch_options: None,
                                proton_version: None,
                                is_hidden: false,
                                tags: Vec::new(),
                            });
                            in_app = true;
                        }
                    }
                }
            }

            // 检测应用块结束
            if line == "}" && in_app {
                if let Some(shortcut) = current.take() {
                    if !shortcut.app_name.is_empty() {
                        shortcuts.push(shortcut);
                    }
                }
                in_app = false;
            }

            // 解析字段
            if let Some(ref mut shortcut) = current {
                if line.starts_with("\"AppName\"") {
                    if let Some(start) = line.find("AppName") {
                        if let Some(q1) = line[start..].find('"') {
                            if let Some(q2) = line[start..].chars().skip(q1 + 1).position(|c| c == '"') {
                                shortcut.app_name = line[start + q1 + 1..start + q1 + 1 + q2].to_string();
                            }
                        }
                    }
                } else if line.starts_with("\"exe\"") {
                    if let Some(start) = line.find("exe") {
                        if let Some(q1) = line[start..].find('"') {
                            if let Some(q2) = line[start..].chars().skip(q1 + 1).position(|c| c == '"') {
                                shortcut.exe_path = line[start + q1 + 1..start + q1 + 1 + q2].to_string();
                            }
                        }
                    }
                } else if line.starts_with("\"StartDir\"") {
                    if let Some(start) = line.find("StartDir") {
                        if let Some(q1) = line[start..].find('"') {
                            if let Some(q2) = line[start..].chars().skip(q1 + 1).position(|c| c == '"') {
                                shortcut.start_dir = line[start + q1 + 1..start + q1 + 1 + q2].to_string();
                            }
                        }
                    }
                } else if line.starts_with("\"LaunchOptions\"") {
                    if let Some(start) = line.find("LaunchOptions") {
                        if let Some(q1) = line[start..].find('"') {
                            if let Some(q2) = line[start..].chars().skip(q1 + 1).position(|c| c == '"') {
                                let opts = line[start + q1 + 1..start + q1 + 1 + q2].to_string();
                                if !opts.is_empty() {
                                    shortcut.launch_options = Some(opts);
                                }
                            }
                        }
                    }
                } else if line.starts_with("\"IsHidden\"") {
                    if let Some(val) = line.chars().filter(|c| c.is_ascii_digit()).next() {
                        shortcut.is_hidden = val == '1';
                    }
                }
            }
        }

        Ok(shortcuts)
    }

    /// 添加快捷方式
    pub fn add_shortcut(&self, game: &GameInfo) -> GameBoxResult<u64> {
        // 生成 App ID
        let app_id = Self::generate_app_id(&game.exe_path.as_deref().unwrap_or(""), &game.name);

        // 读取现有 shortcuts
        let mut shortcuts = self.list_shortcuts()?;

        // 检查是否已存在
        if let Some(existing) = shortcuts.iter_mut().find(|s| s.app_name == game.name) {
            // 更新现有
            existing.exe_path = game.exe_path.clone().unwrap_or_default();
            if let Some(ref exe) = game.exe_path {
                existing.start_dir = PathBuf::from(exe)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
            }
            existing.launch_options = game.launch_options.clone();

            // 保存并返回
            self.save_shortcuts(&shortcuts)?;

            // 设置 Proton
            if let Some(ref proton) = game.proton_version {
                self.set_proton(app_id, proton)?;
            }

            return Ok(app_id);
        }

        // 添加新的快捷方式
        let exe_path = game.exe_path.clone().unwrap_or_default();
        let start_dir = PathBuf::from(&exe_path)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let shortcut = ShortcutInfo {
            app_id,
            app_name: game.name.clone(),
            exe_path,
            start_dir,
            launch_options: game.launch_options.clone(),
            proton_version: game.proton_version.clone(),
            is_hidden: false,
            tags: vec!["GameBox".to_string()],
        };

        shortcuts.push(shortcut);

        // 保存
        self.save_shortcuts(&shortcuts)?;

        // 设置 Proton
        if let Some(ref proton) = game.proton_version {
            self.set_proton(app_id, proton)?;
        }

        Ok(app_id)
    }

    /// 移除快捷方式
    pub fn remove_shortcut(&self, app_id: u64) -> GameBoxResult<()> {
        let mut shortcuts = self.list_shortcuts()?;
        let original_len = shortcuts.len();

        shortcuts.retain(|s| s.app_id != app_id);

        if shortcuts.len() == original_len {
            return Ok(()); // 没找到，不报错
        }

        self.save_shortcuts(&shortcuts)?;

        // 移除 Proton 配置
        self.remove_proton(app_id)?;

        Ok(())
    }

    /// 保存快捷方式到 VDF
    fn save_shortcuts(&self, shortcuts: &[ShortcutInfo]) -> GameBoxResult<()> {
        let data = Self::build_shortcuts_vdf(shortcuts);

        // 确保目录存在
        if let Some(parent) = self.shortcuts_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 原子写入
        let temp_path = self.shortcuts_path.with_extension("tmp");
        fs::write(&temp_path, &data)?;
        fs::rename(&temp_path, &self.shortcuts_path)?;

        Ok(())
    }

    /// 构建 shortcuts.vdf 二进制数据
    fn build_shortcuts_vdf(shortcuts: &[ShortcutInfo]) -> Vec<u8> {
        let mut output = Vec::new();

        // VDF 头部
        writeln!(&mut output, "\"shortcuts\"").ok();
        writeln!(&mut output, "{{").ok();

        for (idx, shortcut) in shortcuts.iter().enumerate() {
            writeln!(&mut output, "\t\"{}\"", idx).ok();
            writeln!(&mut output, "\t{{").ok();
            writeln!(&mut output, "\t\t\"appid\"\t\t\"{}\"", shortcut.app_id).ok();
            writeln!(&mut output, "\t\t\"AppName\"\t\t\"{}\"", Self::escape_vdf_string(&shortcut.app_name)).ok();
            writeln!(&mut output, "\t\t\"exe\"\t\t\"{}\"", Self::escape_vdf_string(&shortcut.exe_path)).ok();
            writeln!(&mut output, "\t\t\"StartDir\"\t\t\"{}\"", Self::escape_vdf_string(&shortcut.start_dir)).ok();

            if let Some(ref opts) = shortcut.launch_options {
                if !opts.is_empty() {
                    writeln!(&mut output, "\t\t\"LaunchOptions\"\t\t\"{}\"", Self::escape_vdf_string(opts)).ok();
                }
            }

            writeln!(&mut output, "\t\t\"icon\"\t\t\"\"").ok();
            writeln!(&mut output, "\t\t\"ShortcutPath\"\t\t\"\"").ok();
            writeln!(&mut output, "\t\t\"IsHidden\"\t\t\"{}\"", if shortcut.is_hidden { "1" } else { "0" }).ok();
            writeln!(&mut output, "\t\t\"AllowDesktopConfig\"\t\t\"1\"").ok();
            writeln!(&mut output, "\t\t\"OpenVR\"\t\t\"0\"").ok();

            // 标签
            if !shortcut.tags.is_empty() {
                writeln!(&mut output, "\t\t\"tags\"").ok();
                writeln!(&mut output, "\t\t{{").ok();
                for (tag_idx, tag) in shortcut.tags.iter().enumerate() {
                    writeln!(&mut output, "\t\t\t\"{}\"\t\t\"{}\"", tag_idx, Self::escape_vdf_string(tag)).ok();
                }
                writeln!(&mut output, "\t\t}}").ok();
            }

            writeln!(&mut output, "\t}}").ok();
        }

        writeln!(&mut output, "}}").ok();

        output
    }

    /// 转义 VDF 字符串
    fn escape_vdf_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
    }

    /// 生成 App ID
    fn generate_app_id(exe_path: &str, app_name: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        exe_path.hash(&mut hasher);
        app_name.hash(&mut hasher);

        let hash = hasher.finish();
        // Steam 快捷方式 App ID 范围：0x100000000 以上
        (hash as u64) | 0x80000000
    }

    /// 设置 Proton 版本
    pub fn set_proton(&self, app_id: u64, proton: &str) -> GameBoxResult<()> {
        if !self.config_vdf_path.exists() {
            return Err(GameBoxError::SetProtonFailed("config.vdf 不存在".to_string()));
        }

        let mut content = fs::read_to_string(&self.config_vdf_path)?;

        let app_id_str = app_id.to_string();

        // 检查是否已有 CompatToolMapping
        let has_compat_mapping = content.contains("CompatToolMapping");

        // 构建要插入/替换的条目
        let entry = format!(
            "\n\t\t\t\t\"{}\"\n\t\t\t\t{{\n\t\t\t\t\t\"name\"\t\t\"{}\"\n\t\t\t\t\t\"config\"\t\t\"\"\n\t\t\t\t\t\"priority\"\t\t\"250\"\n\t\t\t\t}}",
            app_id_str,
            proton
        );

        if has_compat_mapping {
            // 尝试替换现有条目
            let re_pattern = regex::Regex::new(&format!(
                r#"\s*"{}"\s*\{{[^}}]*\}}"#,
                regex::escape(&app_id_str)
            )).map_err(|e| GameBoxError::SetProtonFailed(e.to_string()))?;

            if re_pattern.is_match(&content) {
                content = re_pattern.replace(&content, entry.trim()).to_string();
            } else {
                // 追加到 CompatToolMapping 块
                if let Some(pos) = content.find("CompatToolMapping") {
                    if let Some(brace_pos) = content[pos..].find('{') {
                        let insert_pos = pos + brace_pos + 1;
                        content.insert_str(insert_pos, &entry);
                    }
                }
            }
        } else {
            // 添加 CompatToolMapping 块
            content.push_str(&format!(
                "\n\"CompatToolMapping\"\n{{\n{}}}\n",
                entry
            ));
        }

        // 原子写入
        let temp_path = self.config_vdf_path.with_extension("tmp");
        fs::write(&temp_path, &content)?;
        fs::rename(&temp_path, &self.config_vdf_path)?;

        Ok(())
    }

    /// 移除 Proton 配置
    pub fn remove_proton(&self, app_id: u64) -> GameBoxResult<()> {
        if !self.config_vdf_path.exists() {
            return Ok(());
        }

        let mut content = fs::read_to_string(&self.config_vdf_path)?;

        let app_id_str = app_id.to_string();

        // 移除条目
        let re_pattern = regex::Regex::new(&format!(
            r#"\s*"{}"\s*\{{[^}}]*\}}\s*"#,
            regex::escape(&app_id_str)
        )).map_err(|e| GameBoxError::SetProtonFailed(e.to_string()))?;

        content = re_pattern.replace(&content, "").to_string();

        // 写入
        fs::write(&self.config_vdf_path, &content)?;

        Ok(())
    }
}
