<template>
  <div class="gamebox-view">
    <!-- 页面标题 -->
    <div class="page-header">
      <h2>SteamDeck 游戏盒子</h2>
      <el-button type="primary" @click="showSettings = true">
        <el-icon><Setting /></el-icon>
        设置
      </el-button>
    </div>

    <!-- 标签页 -->
    <el-tabs v-model="activeTab" class="gamebox-tabs">
      <!-- 安装向导 -->
      <el-tab-pane label="安装游戏" name="install">
        <div class="install-panel">
          <!-- 步骤指示器 -->
          <el-steps :active="currentStep" finish-status="success" class="install-steps">
            <el-step title="选择压缩包" />
            <el-step title="设置安装" />
            <el-step title="配置 Steam" />
            <el-step title="完成" />
          </el-steps>

          <!-- 步骤1: 选择压缩包 -->
          <div v-show="currentStep === 0" class="step-content">
            <el-card>
              <template #header>
                <span>选择压缩包</span>
              </template>
              
              <div class="directory-input">
                <el-input
                  v-model="scanDirectory"
                  placeholder="输入目录路径，如 ~/Downloads"
                  clearable
                >
                  <template #append>
                    <el-button @click="handleScanDirectory">扫描</el-button>
                  </template>
                </el-input>
              </div>

              <el-table
                v-if="archives.length > 0"
                :data="archives"
                stripe
                style="width: 100%; margin-top: 16px"
                @row-click="handleSelectArchive"
                :row-class-name="(row: ArchiveInfo) => selectedArchive?.path === row.path ? 'selected-row' : ''"
              >
                <el-table-column prop="name" label="文件名" min-width="200" />
                <el-table-column prop="size" label="大小" width="120" :formatter="formatSize" />
                <el-table-column label="类型" width="120">
                  <template #default="{ row }">
                    <el-tag v-if="row.is_multipart" type="warning" size="small">分卷</el-tag>
                    <el-tag v-else type="info" size="small">普通</el-tag>
                  </template>
                </el-table-column>
              </el-table>

              <el-empty v-else-if="!loading" description="扫描目录以查看压缩包" />

              <div class="step-actions">
                <el-button
                  type="primary"
                  :disabled="!selectedArchive"
                  @click="currentStep = 1"
                >
                  下一步
                </el-button>
              </div>
            </el-card>
          </div>

          <!-- 步骤2: 设置安装 -->
          <div v-show="currentStep === 1" class="step-content">
            <el-card>
              <template #header>
                <span>设置安装选项</span>
              </template>

              <el-form label-width="120px" :model="installForm">
                <el-form-item label="游戏名称">
                  <el-input v-model="installForm.gameName" placeholder="输入游戏名称" />
                </el-form-item>

                <el-form-item label="安装目录">
                  <el-input v-model="installForm.installDir" placeholder="输入安装目录">
                    <template #append>
                      <el-button @click="selectInstallDir">浏览</el-button>
                    </template>
                  </el-input>
                </el-form-item>

                <el-form-item label="主程序" v-if="installForm.exePath">
                  <el-input v-model="installForm.exePath" disabled>
                    <template #append>
                      <el-button @click="changeExe">更改</el-button>
                    </template>
                  </el-input>
                </el-form-item>

                <el-form-item label="删除压缩包">
                  <el-switch v-model="installForm.deleteArchive" />
                </el-form-item>
              </el-form>

              <div class="step-actions">
                <el-button @click="currentStep = 0">上一步</el-button>
                <el-button
                  type="primary"
                  :disabled="!installForm.gameName || !installForm.installDir"
                  @click="handleStartInstall"
                  :loading="isInstalling"
                >
                  开始安装
                </el-button>
              </div>
            </el-card>
          </div>

          <!-- 步骤3: 配置 Steam (安装中) -->
          <div v-show="currentStep === 2 && isInstalling" class="step-content">
            <el-card>
              <template #header>
                <span>安装中...</span>
              </template>

              <div class="install-progress">
                <el-progress
                  :percentage="installProgress"
                  :status="installFailed ? 'exception' : undefined"
                  :format="(p: number) => `${p}%`"
                />
                <p class="progress-message">{{ installMessage }}</p>
                <el-tag v-if="installFailed" type="danger">安装失败</el-tag>
              </div>
            </el-card>
          </div>

          <!-- 步骤3: 配置 Steam (完成) -->
          <div v-show="currentStep === 2 && !isInstalling && installCompleted" class="step-content">
            <el-card>
              <template #header>
                <span>Steam 设置</span>
              </template>

              <el-form label-width="120px">
                <el-form-item label="Proton 版本">
                  <el-select v-model="installForm.protonVersion" placeholder="选择 Proton 版本">
                    <el-option
                      v-for="version in protonVersions"
                      :key="version.name"
                      :label="`${version.display_name} ${version.available ? '' : '(未安装)'}`"
                      :value="version.name"
                      :disabled="!version.available"
                    />
                  </el-select>
                </el-form-item>

                <el-form-item label="启动参数">
                  <el-input
                    v-model="installForm.launchOptions"
                    placeholder="如: MANGOHUD=1 %command%"
                  />
                </el-form-item>

                <el-form-item label="启用 MangoHud">
                  <el-switch
                    v-model="enableMangoHud"
                    @change="handleToggleMangoHud"
                  />
                </el-form-item>
              </el-form>

              <div class="step-actions">
                <el-button @click="handleReset">重新安装</el-button>
                <el-button
                  type="primary"
                  :loading="isAddingToSteam"
                  @click="handleAddToSteam"
                >
                  添加到 Steam
                </el-button>
              </div>
            </el-card>
          </div>

          <!-- 步骤4: 完成 -->
          <div v-show="currentStep === 3" class="step-content">
            <el-card>
              <template #header>
                <span>安装完成</span>
              </template>

              <el-result
                icon="success"
                title="游戏已成功安装"
                :sub-title="`游戏: ${installForm.gameName}`"
              >
                <template #extra>
                  <el-button type="primary" @click="handleReset">安装更多游戏</el-button>
                </template>
              </el-result>
            </el-card>
          </div>
        </div>
      </el-tab-pane>

      <!-- 游戏管理 -->
      <el-tab-pane label="游戏管理" name="games">
        <el-card>
          <template #header>
            <div class="card-header">
              <span>已安装的游戏</span>
              <el-button size="small" @click="loadGames">刷新</el-button>
            </div>
          </template>

          <el-table :data="games" stripe style="width: 100%">
            <el-table-column prop="name" label="游戏名称" min-width="150" />
            <el-table-column prop="install_dir" label="安装目录" min-width="200" show-overflow-tooltip />
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" size="small">
                  {{ getStatusText(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="Steam ID" width="120">
              <template #default="{ row }">
                <span v-if="row.steam_app_id">{{ row.steam_app_id }}</span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="180">
              <template #default="{ row }">
                <el-button
                  v-if="row.status === 'installed' && !row.steam_app_id"
                  type="primary"
                  size="small"
                  @click="openAddToSteamDialog(row)"
                >
                  添加到 Steam
                </el-button>
                <el-button
                  v-if="row.steam_app_id"
                  type="danger"
                  size="small"
                  @click="handleRemoveFromSteam(row.steam_app_id)"
                >
                  移除
                </el-button>
              </template>
            </el-table-column>
          </el-table>

          <el-empty v-if="games.length === 0" description="暂无已安装的游戏" />
        </el-card>
      </el-tab-pane>

      <!-- Steam 快捷方式 -->
      <el-tab-pane label="Steam 快捷方式" name="shortcuts">
        <el-card>
          <template #header>
            <div class="card-header">
              <span>Steam 快捷方式</span>
              <el-button size="small" @click="loadShortcuts">刷新</el-button>
            </div>
          </template>

          <el-table :data="shortcuts" stripe style="width: 100%">
            <el-table-column prop="app_name" label="游戏名称" min-width="150" />
            <el-table-column prop="exe_path" label="可执行文件" min-width="250" show-overflow-tooltip />
            <el-table-column label="Proton" width="150">
              <template #default="{ row }">
                <el-select
                  v-if="row.proton_version"
                  v-model="row.proton_version"
                  size="small"
                  @change="(val: string) => handleChangeProton(row.app_id, val)"
                >
                  <el-option
                    v-for="version in protonVersions"
                    :key="version.name"
                    :label="version.display_name"
                    :value="version.name"
                  />
                </el-select>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="100">
              <template #default="{ row }">
                <el-button
                  type="danger"
                  size="small"
                  @click="handleRemoveFromSteam(row.app_id)"
                >
                  移除
                </el-button>
              </template>
            </el-table-column>
          </el-table>

          <el-empty v-if="shortcuts.length === 0" description="暂无 Steam 快捷方式" />
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <!-- 设置对话框 -->
    <el-dialog v-model="showSettings" title="游戏盒子设置" width="500px">
      <el-form label-width="140px">
        <el-form-item label="默认安装目录">
          <el-input v-model="settingsForm.defaultInstallDir" />
        </el-form-item>
        <el-form-item label="默认 Proton 版本">
          <el-select v-model="settingsForm.defaultProton">
            <el-option
              v-for="version in protonVersions"
              :key="version.name"
              :label="version.display_name"
              :value="version.name"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="解压后自动删除压缩包">
          <el-switch v-model="settingsForm.autoDeleteArchive" />
        </el-form-item>
        <el-form-item label="默认启动参数">
          <el-input v-model="settingsForm.defaultLaunchOptions" placeholder="如: MANGOHUD=1 %command%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showSettings = false">取消</el-button>
        <el-button type="primary" @click="handleSaveSettings">保存</el-button>
      </template>
    </el-dialog>

    <!-- 选择主程序对话框 -->
    <el-dialog v-model="showExeSelector" title="选择游戏主程序" width="700px">
      <el-table
        v-if="executables.length > 0"
        :data="executables"
        stripe
        style="width: 100%"
        @row-click="handleSelectExe"
        highlight-current-row
      >
        <el-table-column prop="name" label="文件名" width="200" />
        <el-table-column prop="path" label="路径" min-width="300" show-overflow-tooltip />
        <el-table-column prop="size" label="大小" width="100" :formatter="formatSize" />
        <el-table-column label="推荐" width="80">
          <template #default="{ row }">
            <el-tag v-if="row.score >= 100" type="success" size="small">推荐</el-tag>
          </template>
        </el-table-column>
      </el-table>
      <el-empty v-else description="未找到可执行文件" />
    </el-dialog>

    <!-- 添加到 Steam 对话框 -->
    <el-dialog v-model="showAddToSteamDialog" title="添加到 Steam" width="500px">
      <el-form label-width="120px">
        <el-form-item label="游戏名称">
          <el-input v-model="addToSteamForm.gameName" />
        </el-form-item>
        <el-form-item label="可执行文件">
          <el-input v-model="addToSteamForm.exePath" />
        </el-form-item>
        <el-form-item label="Proton 版本">
          <el-select v-model="addToSteamForm.protonVersion">
            <el-option
              v-for="version in protonVersions"
              :key="version.name"
              :label="version.display_name"
              :value="version.name"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="启动参数">
          <el-input v-model="addToSteamForm.launchOptions" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="showAddToSteamDialog = false">取消</el-button>
        <el-button type="primary" @click="confirmAddToSteam">确认</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Setting } from '@element-plus/icons-vue'
import { useGameBoxStore } from '@/stores/gamebox'
import { formatFileSize, getStatusText, getStatusType } from '@/api/gamebox'
import type { ArchiveInfo, ExecutableInfo, GameInfo } from '@/api/gamebox'

const store = useGameBoxStore()

// ============ 响应式数据 ============

const activeTab = ref('install')
const currentStep = ref(0)
const showSettings = ref(false)
const showExeSelector = ref(false)
const showAddToSteamDialog = ref(false)

// 扫描目录
const scanDirectory = ref('')

// 选中的压缩包
const selectedArchive = ref<ArchiveInfo | null>(null)

// 安装表单
const installForm = reactive({
  gameName: '',
  installDir: '',
  exePath: '',
  protonVersion: 'proton_experimental',
  launchOptions: '',
  deleteArchive: false
})

// 设置表单
const settingsForm = reactive({
  defaultInstallDir: '',
  defaultProton: '',
  autoDeleteArchive: false,
  defaultLaunchOptions: ''
})

// 添加到 Steam 表单
const addToSteamForm = reactive({
  gameName: '',
  exePath: '',
  protonVersion: 'proton_experimental',
  launchOptions: '',
  gameId: ''
})

// 启用 MangoHud
const enableMangoHud = ref(false)

// 安装状态
const isInstalling = ref(false)
const installCompleted = ref(false)
const installFailed = ref(false)
const installProgress = ref(0)
const installMessage = ref('')
const isAddingToSteam = ref(false)
let pollTimer: ReturnType<typeof setInterval> | null = null

// ============ 计算属性 ============

const archives = computed(() => store.archives)
const executables = computed(() => store.executables)
const games = computed(() => store.games)
const shortcuts = computed(() => store.shortcuts)
const protonVersions = computed(() => store.protonVersions)

// ============ 方法 ============

/**
 * 格式化文件大小
 */
function formatSize(row: { size: number }): string {
  return formatFileSize(row.size)
}

/**
 * 扫描目录
 */
async function handleScanDirectory() {
  if (!scanDirectory.value) {
    ElMessage.warning('请输入目录路径')
    return
  }
  await store.scanDirectory(scanDirectory.value)
}

/**
 * 选择压缩包
 */
function handleSelectArchive(row: ArchiveInfo) {
  selectedArchive.value = row
  // 自动填充游戏名称
  if (!installForm.gameName) {
    installForm.gameName = row.name.replace(/\.(7z|zip|rar|tar\.gz|tar\.xz|tar\.bz2|tgz|tbz2|zst)$/i, '')
  }
  // 自动填充安装目录
  if (!installForm.installDir) {
    installForm.installDir = `${scanDirectory.value}/${installForm.gameName}`
  }
}

/**
 * 开始安装
 */
async function handleStartInstall() {
  if (!selectedArchive.value) {
    ElMessage.warning('请选择压缩包')
    return
  }

  isInstalling.value = true
  installCompleted.value = false
  installFailed.value = false
  installProgress.value = 0
  installMessage.value = '准备安装...'
  currentStep.value = 2

  try {
    const result = await store.startInstall({
      archivePath: selectedArchive.value.path,
      installDir: installForm.installDir,
      gameName: installForm.gameName,
      exePath: installForm.exePath || undefined,
      deleteArchive: installForm.deleteArchive
    })

    installMessage.value = result.message

    // 开始轮询进度
    startPolling(result.task_id)

    // 自动设置 Proton
    if (result.completed && !result.failed) {
      installForm.protonVersion = store.config.default_proton
    }
  } catch (error: any) {
    installFailed.value = true
    installMessage.value = error.message || '安装失败'
    isInstalling.value = false
  }
}

/**
 * 开始轮询安装进度
 */
function startPolling(taskId: string) {
  if (pollTimer) {
    clearInterval(pollTimer)
  }

  pollTimer = setInterval(async () => {
    const progress = await store.pollInstallStatus(taskId)
    
    if (progress) {
      installProgress.value = Math.round(progress.progress)
      installMessage.value = progress.message

      if (progress.completed) {
        clearInterval(pollTimer!)
        pollTimer = null
        isInstalling.value = false

        if (progress.failed) {
          installFailed.value = true
        } else {
          installCompleted.value = true
          // 加载游戏列表
          await store.loadGames()
        }
      }
    }
  }, 1000)
}

/**
 * 选择安装目录
 */
function selectInstallDir() {
  // TODO: 调用文件系统 API 选择目录
  ElMessage.info('功能开发中，请手动输入目录路径')
}

/**
 * 更改主程序
 */
async function changeExe() {
  // 先搜索可执行文件
  await store.searchExecutables(installForm.installDir, 6)
  showExeSelector.value = true
}

/**
 * 选择主程序
 */
function handleSelectExe(row: ExecutableInfo) {
  installForm.exePath = row.path
  showExeSelector.value = false
}

/**
 * 切换 MangoHud
 */
function handleToggleMangoHud(enabled: boolean) {
  if (enabled) {
    installForm.launchOptions = 'MANGOHUD=1 %command%'
  } else {
    installForm.launchOptions = ''
  }
}

/**
 * 添加到 Steam
 */
async function handleAddToSteam() {
  isAddingToSteam.value = true

  try {
    const result = await store.addGameToSteam({
      gameName: installForm.gameName,
      exePath: installForm.exePath,
      protonVersion: installForm.protonVersion,
      launchOptions: installForm.launchOptions
    })

    ElMessage.success(`已添加到 Steam (App ID: ${result.app_id})`)
    currentStep.value = 3
    await store.loadGames()
  } catch (error: any) {
    ElMessage.error(error.message || '添加到 Steam 失败')
  } finally {
    isAddingToSteam.value = false
  }
}

/**
 * 重置安装流程
 */
function handleReset() {
  currentStep.value = 0
  selectedArchive.value = null
  installForm.gameName = ''
  installForm.installDir = ''
  installForm.exePath = ''
  installForm.protonVersion = store.config.default_proton
  installForm.launchOptions = ''
  installForm.deleteArchive = false
  installCompleted.value = false
  installFailed.value = false
  installProgress.value = 0
  installMessage.value = ''
  store.resetInstallTask()
}

/**
 * 打开添加到 Steam 对话框
 */
function openAddToSteamDialog(game: GameInfo) {
  addToSteamForm.gameName = game.name
  addToSteamForm.exePath = game.exe_path || ''
  addToSteamForm.protonVersion = game.proton_version || store.config.default_proton
  addToSteamForm.launchOptions = game.launch_options || ''
  addToSteamForm.gameId = game.id
  showAddToSteamDialog.value = true
}

/**
 * 确认添加到 Steam
 */
async function confirmAddToSteam() {
  try {
    const result = await store.addGameToSteam({
      gameName: addToSteamForm.gameName,
      exePath: addToSteamForm.exePath,
      protonVersion: addToSteamForm.protonVersion,
      launchOptions: addToSteamForm.launchOptions
    })

    ElMessage.success(`已添加到 Steam (App ID: ${result.app_id})`)
    showAddToSteamDialog.value = false
    await store.loadGames()
  } catch (error: any) {
    ElMessage.error(error.message || '添加到 Steam 失败')
  }
}

/**
 * 从 Steam 移除
 */
async function handleRemoveFromSteam(appId: number) {
  try {
    await ElMessageBox.confirm('确定要从 Steam 移除此游戏吗？', '确认', {
      confirmButtonText: '确定',
      cancelButtonText: '取消',
      type: 'warning'
    })

    await store.removeGameFromSteam(appId)
    ElMessage.success('已从 Steam 移除')
    await store.loadGames()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '移除失败')
    }
  }
}

/**
 * 更改 Proton 版本
 */
async function handleChangeProton(appId: number, protonVersion: string) {
  try {
    await store.updateProton(appId, protonVersion)
    ElMessage.success('Proton 版本已更新')
  } catch (error: any) {
    ElMessage.error(error.message || '更新失败')
  }
}

/**
 * 加载游戏列表
 */
async function loadGames() {
  await store.loadGames()
}

/**
 * 加载快捷方式列表
 */
async function loadShortcuts() {
  await store.loadShortcuts()
}

/**
 * 保存设置
 */
async function handleSaveSettings() {
  try {
    await store.saveConfig({
      default_install_dir: settingsForm.defaultInstallDir,
      default_proton: settingsForm.defaultProton,
      auto_delete_archive: settingsForm.autoDeleteArchive,
      default_launch_options: settingsForm.defaultLaunchOptions || undefined
    })
    ElMessage.success('设置已保存')
    showSettings.value = false
  } catch (error: any) {
    ElMessage.error(error.message || '保存失败')
  }
}

// ============ 生命周期 ============

onMounted(async () => {
  await store.initialize()

  // 初始化设置表单
  settingsForm.defaultInstallDir = store.config.default_install_dir
  settingsForm.defaultProton = store.config.default_proton
  settingsForm.autoDeleteArchive = store.config.auto_delete_archive
  settingsForm.defaultLaunchOptions = store.config.default_launch_options || ''
  installForm.protonVersion = store.config.default_proton

  // 默认扫描 Downloads 目录
  scanDirectory.value = '~/Downloads'
})

// 监听安装完成
watch(() => installCompleted.value, (completed) => {
  if (completed) {
    currentStep.value = 2
  }
})
</script>

<style scoped>
.gamebox-view {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.gamebox-tabs {
  background: #fff;
  padding: 16px;
  border-radius: 8px;
}

.install-panel {
  max-width: 800px;
  margin: 0 auto;
}

.install-steps {
  margin-bottom: 32px;
}

.step-content {
  margin-top: 24px;
}

.directory-input {
  margin-bottom: 16px;
}

.step-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
}

.install-progress {
  text-align: center;
  padding: 24px;
}

.progress-message {
  margin: 16px 0;
  color: #666;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

:deep(.selected-row) {
  background-color: #ecf5ff;
  cursor: pointer;
}

:deep(.el-step__title) {
  font-size: 14px;
}
</style>
