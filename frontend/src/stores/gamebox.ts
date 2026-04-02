// GameBox Store
import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {
  GameInfo,
  ArchiveInfo,
  GameBoxConfig,
  InstallProgress,
  ProtonVersion,
  ExecutableInfo,
  ShortcutInfo,
  InstallStep
} from '@/api/gamebox'
import * as api from '@/api/gamebox'

export const useGameBoxStore = defineStore('gamebox', () => {
  // ============ 状态 ============
  
  // 游戏列表
  const games = ref<GameInfo[]>([])
  
  // 压缩包列表
  const archives = ref<ArchiveInfo[]>([])
  
  // 可执行文件列表（当前扫描的）
  const executables = ref<ExecutableInfo[]>([])
  
  // Steam 快捷方式列表
  const shortcuts = ref<ShortcutInfo[]>([])
  
  // 可用的 Proton 版本
  const protonVersions = ref<ProtonVersion[]>([])
  
  // 配置
  const config = ref<GameBoxConfig>({
    default_install_dir: '~/Games',
    default_proton: 'proton_experimental',
    auto_delete_archive: false,
    default_launch_options: undefined
  })
  
  // 当前安装任务
  const currentInstallTask = ref<{
    taskId: string
    progress: InstallProgress | null
    status: 'idle' | 'running' | 'completed' | 'failed'
  }>({
    taskId: '',
    progress: null,
    status: 'idle'
  })
  
  // 当前扫描目录
  const currentDirectory = ref('')
  
  // 加载状态
  const loading = ref(false)
  
  // ============ 计算属性 ============
  
  const installedGames = computed(() => 
    games.value.filter(g => g.status === 'installed')
  )
  
  const pendingArchives = computed(() => archives.value)
  
  const defaultProton = computed(() => 
    protonVersions.value.find(p => p.name === config.value.default_proton)
  )
  
  // ============ 方法 ============
  
  /**
   * 加载配置
   */
  async function loadConfig() {
    try {
      config.value = await api.getGameBoxConfig()
    } catch (error) {
      console.error('加载配置失败:', error)
    }
  }
  
  /**
   * 保存配置
   */
  async function saveConfig(newConfig: Partial<GameBoxConfig>) {
    try {
      config.value = await api.updateGameBoxConfig(newConfig)
    } catch (error) {
      console.error('保存配置失败:', error)
      throw error
    }
  }
  
  /**
   * 加载游戏列表
   */
  async function loadGames() {
    loading.value = true
    try {
      games.value = await api.listGames()
    } catch (error) {
      console.error('加载游戏列表失败:', error)
    } finally {
      loading.value = false
    }
  }
  
  /**
   * 删除游戏
   */
  async function removeGame(id: string) {
    try {
      await api.deleteGame(id)
      games.value = games.value.filter(g => g.id !== id)
    } catch (error) {
      console.error('删除游戏失败:', error)
      throw error
    }
  }
  
  /**
   * 扫描目录
   */
  async function scanDirectory(directory: string) {
    loading.value = true
    currentDirectory.value = directory
    try {
      const result = await api.scanDirectory(directory)
      archives.value = result.archives
    } catch (error) {
      console.error('扫描目录失败:', error)
      archives.value = []
    } finally {
      loading.value = false
    }
  }
  
  /**
   * 加载压缩包列表
   */
  async function loadArchives(directory?: string) {
    loading.value = true
    try {
      archives.value = await api.listArchives(directory)
    } catch (error) {
      console.error('加载压缩包列表失败:', error)
      archives.value = []
    } finally {
      loading.value = false
    }
  }
  
  /**
   * 查找可执行文件
   */
  async function searchExecutables(directory: string, maxDepth?: number) {
    loading.value = true
    try {
      executables.value = await api.findExecutables(directory, maxDepth)
    } catch (error) {
      console.error('查找可执行文件失败:', error)
      executables.value = []
    } finally {
      loading.value = false
    }
  }
  
  /**
   * 加载 Steam 快捷方式
   */
  async function loadShortcuts() {
    try {
      shortcuts.value = await api.listShortcuts()
    } catch (error) {
      console.error('加载快捷方式失败:', error)
      shortcuts.value = []
    }
  }
  
  /**
   * 加载 Proton 版本列表
   */
  async function loadProtonVersions() {
    try {
      protonVersions.value = await api.listProtonVersions()
    } catch (error) {
      console.error('加载 Proton 版本失败:', error)
      protonVersions.value = []
    }
  }
  
  /**
   * 开始安装
   */
  async function startInstall(request: {
    archivePath: string
    installDir: string
    gameName: string
    exePath?: string
    protonVersion?: string
    launchOptions?: string
    deleteArchive: boolean
  }) {
    try {
      const progress = await api.startInstall({
        archive_path: request.archivePath,
        install_dir: request.installDir,
        game_name: request.gameName,
        exe_path: request.exePath,
        proton_version: request.protonVersion,
        launch_options: request.launchOptions,
        delete_archive: request.deleteArchive
      })
      
      currentInstallTask.value = {
        taskId: progress.task_id,
        progress,
        status: progress.completed ? (progress.failed ? 'failed' : 'completed') : 'running'
      }
      
      return progress
    } catch (error) {
      console.error('开始安装失败:', error)
      throw error
    }
  }
  
  /**
   * 获取安装进度
   */
  async function pollInstallStatus(taskId: string): Promise<InstallProgress | null> {
    try {
      const progress = await api.getInstallStatus(taskId)
      currentInstallTask.value.progress = progress
      currentInstallTask.value.status = progress.completed 
        ? (progress.failed ? 'failed' : 'completed') 
        : 'running'
      return progress
    } catch (error) {
      console.error('获取安装进度失败:', error)
      return null
    }
  }
  
  /**
   * 添加游戏到 Steam
   */
  async function addGameToSteam(request: {
    gameName: string
    exePath: string
    protonVersion?: string
    launchOptions?: string
  }) {
    try {
      const result = await api.addToSteam({
        game_name: request.gameName,
        exe_path: request.exePath,
        proton_version: request.protonVersion,
        launch_options: request.launchOptions
      })
      
      // 刷新快捷方式列表
      await loadShortcuts()
      
      return result
    } catch (error) {
      console.error('添加到 Steam 失败:', error)
      throw error
    }
  }
  
  /**
   * 从 Steam 移除游戏
   */
  async function removeGameFromSteam(appId: number) {
    try {
      await api.removeFromSteam(appId)
      shortcuts.value = shortcuts.value.filter(s => s.app_id !== appId)
    } catch (error) {
      console.error('从 Steam 移除失败:', error)
      throw error
    }
  }
  
  /**
   * 设置 Proton 版本
   */
  async function updateProton(appId: number, protonVersion: string) {
    try {
      await api.setProton(appId, protonVersion)
      
      // 更新快捷方式列表中的版本
      const shortcut = shortcuts.value.find(s => s.app_id === appId)
      if (shortcut) {
        shortcut.proton_version = protonVersion
      }
    } catch (error) {
      console.error('设置 Proton 版本失败:', error)
      throw error
    }
  }
  
  /**
   * 重置安装任务状态
   */
  function resetInstallTask() {
    currentInstallTask.value = {
      taskId: '',
      progress: null,
      status: 'idle'
    }
  }
  
  // ============ 初始化 ============
  
  async function initialize() {
    await Promise.all([
      loadConfig(),
      loadGames(),
      loadProtonVersions(),
      loadShortcuts()
    ])
  }
  
  return {
    // 状态
    games,
    archives,
    executables,
    shortcuts,
    protonVersions,
    config,
    currentInstallTask,
    currentDirectory,
    loading,
    
    // 计算属性
    installedGames,
    pendingArchives,
    defaultProton,
    
    // 方法
    loadConfig,
    saveConfig,
    loadGames,
    removeGame,
    scanDirectory,
    loadArchives,
    searchExecutables,
    loadShortcuts,
    loadProtonVersions,
    startInstall,
    pollInstallStatus,
    addGameToSteam,
    removeGameFromSteam,
    updateProton,
    resetInstallTask,
    initialize
  }
})
