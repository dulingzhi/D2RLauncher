<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import AccountCard from './components/AccountCard.vue'
import AccountModal from './components/AccountModal.vue'
import SettingsModal from './components/SettingsModal.vue'
import ToastContainer from './components/ToastContainer.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import { useToast } from './composables/useToast'
import type { Account } from './types'

const { success, error: toastError, warning, info } = useToast()

interface GameInstance {
  account_id: string
  account_name: string
  region: string
  process_id: number
  window_title: string
  start_time: string
  total_seconds: number
}

interface GameStatus {
  running_instances: GameInstance[]
  total_running: number
}

const accounts = ref<Account[]>([])
const showModal = ref(false)
const showSettings = ref(false)
const editingAccount = ref<Account | null>(null)
const batchLaunching = ref(false)
const runningGames = ref<Map<string, GameInstance>>(new Map())
const launchQueue = ref<string[]>([])
const currentLaunching = ref<string | null>(null)
const waitingForLogin = ref<{ accountId: string; accountName: string } | null>(null)

const confirmRef = ref<InstanceType<typeof ConfirmDialog> | null>(null)

let unlistenGameStatus: UnlistenFn | null = null
let unlistenLoginComplete: UnlistenFn | null = null

async function loadAccounts() {
  accounts.value = await invoke('get_accounts')
}

const runningCount = computed(() => runningGames.value.size)

onMounted(async () => {
  // 检查更新（仅生产环境）
  if (import.meta.env.PROD) {
    try {
      const update = await check()
      if (update) {
        const ok = await confirmRef.value?.confirm({
          title: '发现新版本',
          message: `版本 ${update.version}\n\n更新内容：\n${update.body}\n\n是否立即下载并安装？`,
          confirmText: '下载更新',
        })
        if (ok) {
          info('正在下载更新...')
          let downloaded = 0
          let contentLength = 0

          await update.downloadAndInstall((event) => {
            switch (event.event) {
              case 'Started':
                contentLength = event.data.contentLength || 0
                break
              case 'Progress':
                downloaded += event.data.chunkLength
                const percent = contentLength > 0 ? ((downloaded / contentLength) * 100).toFixed(0) : '0'
                info(`下载进度: ${percent}%`)
                break
              case 'Finished':
                info('下载完成，准备安装...')
                break
            }
          })

          success('更新安装完成，正在重启...')
          await relaunch()
        }
      }
    } catch {
      // 静默处理更新检查失败
    }
  }

  await loadAccounts()

  unlistenGameStatus = await listen<GameStatus>('game_status_update', (event) => {
    const newMap = new Map<string, GameInstance>()
    event.payload.running_instances.forEach((instance) => {
      newMap.set(instance.account_id, instance)
    })
    runningGames.value = newMap

    // 等待登录的账号游戏进程消失 → 清除遮罩
    if (currentLaunching.value && waitingForLogin.value) {
      const isGameRunning = newMap.has(currentLaunching.value)

      if (!isGameRunning) {
        waitingForLogin.value = null
        currentLaunching.value = null

        if (batchLaunching.value && launchQueue.value.length > 0) {
          setTimeout(() => processLaunchQueue(), 1000)
        } else if (batchLaunching.value) {
          batchLaunching.value = false
          warning('⚠️ 启动中断（游戏被关闭）')
        }
      }
    }
  })

  unlistenLoginComplete = await listen<{
    account_id: string
    account_name: string
    success: boolean
    timeout?: boolean
    error?: string
  }>('login_complete', (event) => {
    if (event.payload.account_id === currentLaunching.value) {
      if (event.payload.success) {
        // 正常流程
      } else if (event.payload.timeout) {
        // 超时，继续下一个
      } else {
        // 失败
      }

      waitingForLogin.value = null
      currentLaunching.value = null
      processLaunchQueue()
    }
  })

  try {
    await invoke('start_game_monitoring')
  } catch (e) {
    console.error('启动游戏监控失败:', e)
  }
})

onUnmounted(async () => {
  unlistenGameStatus?.()
  unlistenLoginComplete?.()
  try {
    await invoke('stop_game_monitoring')
  } catch {
    // ignore
  }
})

function openAddModal() {
  editingAccount.value = null
  showModal.value = true
}

function openEditModal(account: Account) {
  editingAccount.value = account
  showModal.value = true
}

async function handleSave(data: Omit<Account, 'id' | 'encrypted_token' | 'token_set_at'>) {
  try {
    if (editingAccount.value) {
      await invoke('update_account', { account: { ...editingAccount.value, ...data } })
      success('账号已更新')
    } else {
      await invoke('add_account', {
        label: data.label,
        customArgs: data.custom_args,
        windowX: data.window_x,
        windowY: data.window_y,
        windowWidth: data.window_width,
        windowHeight: data.window_height,
      })
      success('账号已添加')
    }
    showModal.value = false
    await loadAccounts()
  } catch (e) {
    toastError(`保存失败: ${e}`)
  }
}

async function handleDelete(id: string) {
  const ok = await confirmRef.value?.confirm({
    title: '删除账号',
    message: '确认删除该账号？此操作不可撤销。',
    confirmText: '删除',
    danger: true,
  })
  if (!ok) return
  try {
    await invoke('delete_account', { id })
    success('账号已删除')
    await loadAccounts()
  } catch (e) {
    toastError(`删除失败: ${e}`)
  }
}

async function handleKillProcess(processId: number, accountName: string) {
  const ok = await confirmRef.value?.confirm({
    title: '关闭进程',
    message: `确定要关闭「${accountName}」的游戏进程吗？\nPID: ${processId}`,
    confirmText: '关闭',
    danger: true,
  })
  if (!ok) return
  try {
    await invoke('kill_game_process', { processId })
    success(`已关闭「${accountName}」进程`)
    await loadAccounts()
  } catch (e) {
    toastError(`关闭失败: ${e}`)
  }
}

async function handleLaunchAccount(account: Account) {
  try {
    const settings: any = await invoke('get_settings')

    if (!settings.game_path || settings.game_path.trim() === '') {
      warning('请先设置 D2R.exe 所在目录')
      showSettings.value = true
      return
    }

    if (settings.wait_for_login) {
      currentLaunching.value = account.id
      waitingForLogin.value = { accountId: account.id, accountName: account.label }
    }

    await invoke('launch_account', { accountId: account.id, gamePath: settings.game_path })

    if (!settings.wait_for_login) {
      currentLaunching.value = null
      waitingForLogin.value = null
    }
  } catch (e) {
    toastError(`启动失败: ${e}`)
    currentLaunching.value = null
    waitingForLogin.value = null
  }
}

async function processLaunchQueue() {
  if (launchQueue.value.length === 0) {
    batchLaunching.value = false
    success('✅ 全部启动完成')
    waitingForLogin.value = null
    return
  }

  const accountId = launchQueue.value.shift()!
  currentLaunching.value = accountId

  const account = accounts.value.find((a) => a.id === accountId)
  if (!account) {
    processLaunchQueue()
    return
  }

  info(`正在启动: ${account.label}... (剩余 ${launchQueue.value.length} 个)`)

  try {
    const settings: any = await invoke('get_settings')
    await invoke('launch_account', { accountId, gamePath: settings.game_path })

    if (settings.wait_for_login) {
      waitingForLogin.value = { accountId, accountName: account.label }
    }

    if (!settings.wait_for_login) {
      await new Promise((resolve) => setTimeout(resolve, settings.launch_delay_secs * 1000))
      currentLaunching.value = null
      processLaunchQueue()
    }
  } catch (e) {
    toastError(`❌ ${account.label} 启动失败: ${e}`)
    currentLaunching.value = null
    waitingForLogin.value = null
    setTimeout(() => processLaunchQueue(), 2000)
  }
}

async function launchAll() {
  const ids = accounts.value.filter((a) => a.encrypted_token).map((a) => a.id)

  if (!ids.length) {
    warning('没有已配置 Token 的账号')
    return
  }

  if (batchLaunching.value) {
    info('已经在批量启动中...')
    return
  }

  try {
    const settings: any = await invoke('get_settings')
    if (!settings.game_path || settings.game_path.trim() === '') {
      warning('请先设置 D2R.exe 所在目录')
      showSettings.value = true
      return
    }
  } catch {
    toastError('获取设置失败')
    return
  }

  launchQueue.value = [...ids]
  batchLaunching.value = true
  info(`准备启动 ${ids.length} 个账号...`)
  processLaunchQueue()
}

const readyCount = () => accounts.value.filter((a) => a.encrypted_token).length
</script>

<template>
  <div class="app">
    <!-- 顶栏 -->
    <header class="topbar">
      <div class="topbar-left">
        <div class="app-title">
          <span class="title-text">D2R CN 多开启动器</span>
        </div>
      </div>

      <div class="topbar-center">
        <span class="status-badge" v-if="runningCount > 0">
          🎮 {{ runningCount }} 运行中
        </span>
        <span class="status-badge status-ready" v-else-if="readyCount() > 0">
          {{ readyCount() }} 就绪
        </span>
      </div>

      <div class="topbar-right">
        <button class="btn-icon" @click="showSettings = true" title="设置">⚙️</button>
      </div>
    </header>

    <!-- 主内容区 -->
    <main class="main-content">
      <!-- 操作栏 -->
      <div class="toolbar">
        <div class="toolbar-left">
          <span class="toolbar-meta">
            共 <strong>{{ accounts.length }}</strong> 个账号，
            <span class="text-green">{{ readyCount() }} 个已配置</span>
          </span>
        </div>
        <div class="toolbar-right">
          <button class="btn btn-secondary" @click="openAddModal">+ 添加</button>
          <button
            class="btn btn-primary"
            :disabled="batchLaunching || readyCount() === 0"
            @click="launchAll"
          >
            {{ batchLaunching ? '启动中...' : '🚀 全部启动' }}
          </button>
        </div>
      </div>

      <!-- 空状态 -->
      <div v-if="accounts.length === 0" class="empty-state">
        <div class="empty-icon">📂</div>
        <p>暂无账号</p>
        <p class="empty-hint">点击「+ 添加」开始</p>
      </div>

      <!-- 账号卡片列表 -->
      <AccountCard
        v-for="account in accounts"
        :key="account.id"
        :account="account"
        :running-game="runningGames.get(account.id)"
        @refresh="loadAccounts"
        @edit="openEditModal"
        @delete="handleDelete"
        @launch="handleLaunchAccount"
        @kill="handleKillProcess"
      />
    </main>

    <!-- 添加/编辑弹窗 -->
    <AccountModal
      :visible="showModal"
      :initial="editingAccount"
      @close="showModal = false"
      @save="handleSave"
    />

    <!-- 设置弹窗 -->
    <SettingsModal
      :visible="showSettings"
      @close="showSettings = false"
      @accounts-updated="loadAccounts"
    />

    <!-- 等待登录遮罩 -->
    <Transition name="fade">
      <div v-if="waitingForLogin" class="login-overlay">
        <div class="login-overlay-content">
          <div class="spinner"></div>
          <div class="login-text">
            <div class="login-title">🔐 等待登录</div>
            <div class="login-account">{{ waitingForLogin.accountName }}</div>
            <div class="login-hint">请在游戏中完成登录并到达角色选择界面</div>
            <div class="login-warning">⚠️ 请勿关闭游戏或启动其他账号</div>
          </div>
        </div>
      </div>
    </Transition>

    <!-- 全局 Toast -->
    <ToastContainer />

    <!-- 全局确认弹窗 -->
    <ConfirmDialog ref="confirmRef" />
  </div>
</template>

<style>
/* ========== 全局重置 ========== */
* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  background: #0d1117;
  color: #e2e8f0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  font-size: 13px;
  height: 100vh;
  overflow: hidden;
  user-select: none;
  -webkit-user-select: none;
}

input, textarea { user-select: text; -webkit-user-select: text; }

::-webkit-scrollbar { width: 5px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: #2d3050; border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: #3d4060; }

#app { height: 100vh; display: flex; flex-direction: column; }
</style>

<style scoped>
.app { display: flex; flex-direction: column; height: 100vh; }

/* ========== 顶栏 ========== */
.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px;
  height: 44px;
  background: #161b2e;
  border-bottom: 1px solid #1e2540;
  flex-shrink: 0;
}

.topbar-left { display: flex; align-items: center; gap: 8px; }

.app-title {
  font-size: 14px;
  font-weight: 700;
  color: #f1f5f9;
  letter-spacing: -0.01em;
}

.topbar-center { display: flex; align-items: center; }

.status-badge {
  font-size: 12px;
  color: #64748b;
  padding: 3px 10px;
  border-radius: 10px;
  background: rgba(100, 116, 139, 0.1);
  border: 1px solid rgba(100, 116, 139, 0.2);
}

.status-badge.status-ready {
  color: #86efac;
  background: rgba(74, 222, 128, 0.08);
  border-color: rgba(74, 222, 128, 0.2);
}

.topbar-right { display: flex; align-items: center; gap: 4px; }

.btn-icon {
  background: transparent;
  border: none;
  cursor: pointer;
  font-size: 16px;
  padding: 6px 8px;
  border-radius: 6px;
  transition: background 0.15s;
}

.btn-icon:hover { background: rgba(255, 255, 255, 0.06); }

/* ========== 主内容 ========== */
.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 14px 16px;
}

/* ========== 工具栏 ========== */
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.toolbar-meta {
  font-size: 12px;
  color: #64748b;
}

.toolbar-meta strong {
  color: #cbd5e1;
  font-weight: 600;
}

.text-green { color: #86efac; }

.toolbar-right { display: flex; gap: 6px; }

/* ========== 按钮 ========== */
.btn {
  padding: 6px 14px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: all 0.15s;
  white-space: nowrap;
}

.btn:disabled { opacity: 0.45; cursor: not-allowed; }

.btn-primary { background: #3b82f6; color: #fff; }
.btn-primary:not(:disabled):hover { background: #2563eb; }

.btn-secondary { background: #1e2a3a; color: #94a3b8; border: 1px solid #2d3050; }
.btn-secondary:not(:disabled):hover { background: #253348; color: #e2e8f0; }

/* ========== 空状态 ========== */
.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: #475569;
}

.empty-icon { font-size: 40px; margin-bottom: 12px; opacity: 0.5; }

.empty-state p {
  margin: 0;
  font-size: 14px;
}

.empty-hint {
  font-size: 12px !important;
  margin-top: 4px !important;
  color: #374151 !important;
}

/* ========== 等待登录遮罩 ========== */
.login-overlay {
  position: fixed;
  inset: 0;
  background: rgba(13, 17, 23, 0.85);
  backdrop-filter: blur(8px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.login-overlay-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 20px;
  padding: 40px 56px;
  background: linear-gradient(135deg, rgba(30, 41, 59, 0.95), rgba(15, 23, 42, 0.95));
  border-radius: 14px;
  border: 1px solid rgba(59, 130, 246, 0.3);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.spinner {
  width: 48px;
  height: 48px;
  border: 3px solid rgba(59, 130, 246, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.login-text {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  text-align: center;
}

.login-title {
  font-size: 18px;
  font-weight: 700;
  color: #f1f5f9;
}

.login-account {
  font-size: 24px;
  font-weight: 800;
  color: #60a5fa;
  text-shadow: 0 0 14px rgba(96, 165, 250, 0.4);
}

.login-hint {
  font-size: 13px;
  color: #94a3b8;
  margin-top: 4px;
}

.login-warning {
  font-size: 12px;
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.08);
  padding: 6px 14px;
  border-radius: 6px;
  border: 1px solid rgba(251, 191, 36, 0.15);
  margin-top: 4px;
}

/* ========== 淡入淡出 ========== */
.fade-enter-active,
.fade-leave-active { transition: opacity 0.25s ease; }
.fade-enter-from,
.fade-leave-to { opacity: 0; }
</style>
