// GameBox API 封装
import { apiClient } from './client'

// ============ 类型定义 ============

export interface GameInfo {
  id: string
  name: string
  install_dir: string
  exe_path?: string
  proton_version?: string
  launch_options?: string
  steam_app_id?: number
  status: GameStatus
  created_at: number
  updated_at: number
}

export type GameStatus = 'pending' | 'extracting' | 'extracted' | 'configuring' | 'installed' | 'failed'

export interface ExecutableInfo {
  path: string
  name: string
  extension: string
  size: number
  score: number
  depth: number
}

export interface ArchiveInfo {
  path: string
  name: string
  size: number
  is_multipart: boolean
  part_count?: number
}

export interface InstallProgress {
  task_id: string
  step: InstallStep
  progress: number
  message: string
  completed: boolean
  failed: boolean
  error?: string
}

export type InstallStep = 'preparing' | 'extracting' | 'finding_exe' | 'adding_to_steam' | 'configuring_proton' | 'completed' | 'failed'

export interface ShortcutInfo {
  app_id: number
  app_name: string
  exe_path: string
  start_dir: string
  launch_options?: string
  proton_version?: string
  is_hidden: boolean
  tags: string[]
}

export interface ProtonVersion {
  name: string
  display_name: string
  available: boolean
}

export interface GameBoxConfig {
  default_install_dir: string
  default_proton: string
  auto_delete_archive: boolean
  default_launch_options?: string
  steam_root?: string
}

export interface InstallRequest {
  archive_path: string
  install_dir: string
  game_name: string
  exe_path?: string
  proton_version?: string
  launch_options?: string
  delete_archive: boolean
}

export interface ScanResult {
  directory: string
  archives: ArchiveInfo[]
}

export interface AddToSteamRequest {
  game_name: string
  exe_path: string
  proton_version?: string
  launch_options?: string
}

export interface AddToSteamResponse {
  app_id: number
  success: boolean
}

// ============ API 函数 ============

/**
 * 获取 GameBox 配置
 */
export async function getGameBoxConfig(): Promise<GameBoxConfig> {
  return apiClient.get('/gamebox/config')
}

/**
 * 更新 GameBox 配置
 */
export async function updateGameBoxConfig(config: Partial<GameBoxConfig>): Promise<GameBoxConfig> {
  return apiClient.put('/gamebox/config', config)
}

/**
 * 获取游戏列表
 */
export async function listGames(): Promise<GameInfo[]> {
  return apiClient.get('/gamebox/games')
}

/**
 * 获取单个游戏信息
 */
export async function getGame(id: string): Promise<{ game?: GameInfo; found: boolean }> {
  return apiClient.get(`/gamebox/games/${id}`)
}

/**
 * 删除游戏记录
 */
export async function deleteGame(id: string): Promise<{ success: boolean; removed: boolean }> {
  return apiClient.delete(`/gamebox/games/${id}`)
}

/**
 * 扫描目录中的压缩包
 */
export async function scanDirectory(directory: string): Promise<ScanResult> {
  return apiClient.post('/gamebox/scan', { directory })
}

/**
 * 获取目录下的压缩包列表
 */
export async function listArchives(directory?: string): Promise<ArchiveInfo[]> {
  return apiClient.get('/gamebox/archives', { params: { directory } })
}

/**
 * 开始安装流程
 */
export async function startInstall(request: InstallRequest): Promise<InstallProgress> {
  return apiClient.post('/gamebox/install/start', request)
}

/**
 * 获取安装进度
 */
export async function getInstallStatus(taskId: string): Promise<InstallProgress> {
  return apiClient.get(`/gamebox/install/status/${taskId}`)
}

/**
 * 查找可执行文件
 */
export async function findExecutables(directory: string, maxDepth?: number): Promise<ExecutableInfo[]> {
  return apiClient.post('/gamebox/executables', { directory, max_depth: maxDepth ?? 6 })
}

/**
 * 获取 Steam 快捷方式列表
 */
export async function listShortcuts(): Promise<ShortcutInfo[]> {
  return apiClient.get('/gamebox/steam/shortcuts')
}

/**
 * 添加游戏到 Steam
 */
export async function addToSteam(request: AddToSteamRequest): Promise<AddToSteamResponse> {
  return apiClient.post('/gamebox/steam/add', request)
}

/**
 * 从 Steam 移除游戏
 */
export async function removeFromSteam(appId: number): Promise<{ success: boolean; app_id: number }> {
  return apiClient.delete(`/gamebox/steam/remove/${appId}`)
}

/**
 * 获取可用的 Proton 版本列表
 */
export async function listProtonVersions(): Promise<ProtonVersion[]> {
  return apiClient.get('/gamebox/proton/versions')
}

/**
 * 设置游戏的 Proton 版本
 */
export async function setProton(appId: number, protonVersion: string): Promise<{ success: boolean }> {
  return apiClient.put('/gamebox/proton/set', { app_id: appId, proton_version: protonVersion })
}

// ============ 工具函数 ============

/**
 * 格式化文件大小
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

/**
 * 获取状态显示文本
 */
export function getStatusText(status: GameStatus): string {
  const statusMap: Record<GameStatus, string> = {
    pending: '待解压',
    extracting: '解压中',
    extracted: '待配置',
    configuring: '配置中',
    installed: '已安装',
    failed: '安装失败'
  }
  return statusMap[status] || status
}

/**
 * 获取状态类型
 */
export function getStatusType(status: GameStatus): 'success' | 'warning' | 'danger' | 'info' {
  const typeMap: Record<GameStatus, 'success' | 'warning' | 'danger' | 'info'> = {
    pending: 'info',
    extracting: 'warning',
    extracted: 'warning',
    configuring: 'info',
    installed: 'success',
    failed: 'danger'
  }
  return typeMap[status] || 'info'
}
