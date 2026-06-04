<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { check } from '@tauri-apps/plugin-updater'
import { relaunch } from '@tauri-apps/plugin-process'
import AccountCard from './components/AccountCard.vue'
import AccountModal from './components/AccountModal.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import type { Account } from './types'

type Tab = 'accounts' | 'settings'

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

const tab = ref<Tab>('accounts')
const accounts = ref<Account[]>([])
const showModal = ref(false)
const editingAccount = ref<Account | null>(null)
const batchLaunching = ref(false)
const batchMsg = ref('')
const runningGames = ref<Map<string, GameInstance>>(new Map())
const launchQueue = ref<string[]>([])  // 启动队列
const currentLaunching = ref<string | null>(null)  // 当前正在启动的账号ID
const waitingForLogin = ref<{ accountId: string; accountName: string } | null>(null)  // 正在等待登录的账号

let unlistenGameStatus: UnlistenFn | null = null
let unlistenLoginComplete: UnlistenFn | null = null

async function loadAccounts() {
  accounts.value = await invoke('get_accounts')
}

const runningCount = computed(() => runningGames.value.size)

onMounted(async () => {
  // 检查更新（仅在生产环境）
  if (import.meta.env.PROD) {
    try {
      console.log('🔍 正在检查更新...')
      const update = await check()
      if (update) {
        console.log(`🆕 发现新版本 ${update.version}，当前版本需要更新`)
        const confirmed = confirm(
          `发现新版本 ${update.version}！\n\n更新内容：\n${update.body}\n\n是否立即下载并安装？`
        )
        
        if (confirmed) {
          console.log('⏬ 开始下载更新...')
          let downloaded = 0
          let contentLength = 0
          
          await update.downloadAndInstall((event) => {
            switch (event.event) {
              case 'Started':
                contentLength = event.data.contentLength || 0
                console.log(`📦 开始下载，文件大小: ${(contentLength / 1024 / 1024).toFixed(2)} MB`)
                break
              case 'Progress':
                downloaded += event.data.chunkLength
                const percent = contentLength > 0 ? ((downloaded / contentLength) * 100).toFixed(0) : '0'
                console.log(`⏬ 下载进度: ${percent}%`)
                break
              case 'Finished':
                console.log('✅ 下载完成，准备安装...')
                break
            }
          })
          
          console.log('🔄 更新安装完成，正在重启应用...')
          await relaunch()
        }
      } else {
        console.log('✅ 当前已是最新版本')
      }
    } catch (error) {
      // 静默处理更新检查失败（可能是网络问题或尚未发布版本）
      console.log('ℹ️ 无法检查更新（可能是网络问题或首次发布）')
    }
  } else {
    console.log('ℹ️ 开发模式下跳过更新检查')
  }
  
  await loadAccounts()
  
  // 监听游戏状态更新
  unlistenGameStatus = await listen<GameStatus>('game_status_update', (event) => {
    const newMap = new Map<string, GameInstance>()
    event.payload.running_instances.forEach(instance => {
      newMap.set(instance.account_id, instance)
    })
    runningGames.value = newMap
    
    // 检测：如果正在等待登录的账号游戏进程消失了，清除遮罩层
    if (currentLaunching.value && waitingForLogin.value) {
      const isGameRunning = newMap.has(currentLaunching.value)
      
      // 基于 PID 监控，游戏进程立即可见，无需宽容期
      if (!isGameRunning) {
        console.log(`⚠️ 检测到正在等待登录的游戏 ${waitingForLogin.value.accountName} 已关闭，清除遮罩层`)
        
        waitingForLogin.value = null
        currentLaunching.value = null
        
        // 如果在批量启动中，继续启动下一个
        if (batchLaunching.value && launchQueue.value.length > 0) {
          console.log('继续启动队列中的下一个账号...')
          setTimeout(() => processLaunchQueue(), 1000)
        } else if (batchLaunching.value) {
          // 队列为空，结束批量启动
          batchLaunching.value = false
          batchMsg.value = '⚠️ 启动中断（游戏被关闭）'
          setTimeout(() => (batchMsg.value = ''), 3000)
        }
      }
    }
  })
  
  // 监听登录完成事件（用于队列启动）
  unlistenLoginComplete = await listen<{
    account_id: string
    account_name: string
    success: boolean
    timeout?: boolean
    error?: string
  }>('login_complete', (event) => {
    console.log('📥 收到登录完成事件:', event.payload)
    
    if (event.payload.account_id === currentLaunching.value) {
      if (event.payload.success) {
        console.log(`✅ ${event.payload.account_name} 登录完成`)
      } else if (event.payload.timeout) {
        console.log(`⏰ ${event.payload.account_name} 登录检测超时，继续启动下一个`)
      } else {
        console.log(`⚠️ ${event.payload.account_name} 登录检测失败: ${event.payload.error}`)
      }
      
      // 清除等待登录状态
      waitingForLogin.value = null
      
      // 启动队列中的下一个账号
      currentLaunching.value = null
      processLaunchQueue()
    }
  })
  
  // 启动游戏监控
  try {
    await invoke('start_game_monitoring')
    console.log('🎮 游戏监控已启动')
  } catch (error) {
    console.error('启动游戏监控失败:', error)
  }
})

onUnmounted(async () => {
  if (unlistenGameStatus) {
    unlistenGameStatus()
  }
  if (unlistenLoginComplete) {
    unlistenLoginComplete()
  }
  try {
    await invoke('stop_game_monitoring')
  } catch (error) {
    console.error('停止游戏监控失败:', error)
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
  if (editingAccount.value) {
    await invoke('update_account', {
      account: { ...editingAccount.value, ...data },
    })
  } else {
    await invoke('add_account', {
      label: data.label,
      email: data.email,
      customArgs: data.custom_args,
      windowX: data.window_x,
      windowY: data.window_y,
      windowWidth: data.window_width,
      windowHeight: data.window_height,
    })
  }
  showModal.value = false
  await loadAccounts()
}

async function handleDelete(id: string) {
  if (!confirm('确认删除该账号？')) return
  await invoke('delete_account', { id })
  await loadAccounts()
}

// 处理单个账号启动（从 AccountCard 发来的事件）
async function handleLaunchAccount(account: Account) {
  try {
    const settings: any = await invoke('get_settings')
    
    // 检查游戏路径是否已设置
    if (!settings.game_path || settings.game_path.trim() === '') {
      alert('⚠️ 请先设置 D2R.exe 所在目录！')
      tab.value = 'settings'
      return
    }
    
    // 显示遮罩层（如果启用了登录检测）
    if (settings.wait_for_login) {
      currentLaunching.value = account.id
      waitingForLogin.value = {
        accountId: account.id,
        accountName: account.label
      }
    }
    
    await invoke('launch_account', {
      accountId: account.id,
      gamePath: settings.game_path,
    })
    
    // 如果禁用了登录检测，立即清除状态
    if (!settings.wait_for_login) {
      currentLaunching.value = null
      waitingForLogin.value = null
    }
    // 如果启用了登录检测，等待 login_complete 事件来清除状态
  } catch (e) {
    console.error('启动失败:', e)
    alert(`启动失败: ${e}`)
    currentLaunching.value = null
    waitingForLogin.value = null
  }
}

// 处理启动队列
async function processLaunchQueue() {
  if (launchQueue.value.length === 0) {
    batchLaunching.value = false
    batchMsg.value = '✅ 全部启动完成'
    waitingForLogin.value = null  // 清除等待状态
    setTimeout(() => (batchMsg.value = ''), 3000)
    return
  }
  
  // 取出队列中的第一个账号
  const accountId = launchQueue.value.shift()!
  currentLaunching.value = accountId
  
  const account = accounts.value.find(a => a.id === accountId)
  if (!account) {
    console.error('账号未找到:', accountId)
    processLaunchQueue()  // 继续下一个
    return
  }
  
  const remaining = launchQueue.value.length
  batchMsg.value = `正在启动: ${account.label}... (剩余 ${remaining} 个)`
  
  try {
    const settings: any = await invoke('get_settings')
    await invoke('launch_account', {
      accountId: accountId,
      gamePath: settings.game_path,
    })
    
    // 如果启用了登录检测，显示等待遮罩
    if (settings.wait_for_login) {
      waitingForLogin.value = {
        accountId: accountId,
        accountName: account.label
      }
    }
    
    // 如果禁用了登录检测，延迟后启动下一个
    if (!settings.wait_for_login) {
      console.log(`⚠️ 已禁用登录检测，${settings.launch_delay_secs}秒后启动下一个`)
      await new Promise(resolve => setTimeout(resolve, settings.launch_delay_secs * 1000))
      currentLaunching.value = null
      processLaunchQueue()
    }
    // 如果启用了登录检测，等待 login_complete 事件触发 processLaunchQueue
  } catch (e) {
    console.error('启动失败:', e)
    batchMsg.value = `❌ ${account.label} 启动失败: ${e}`
    currentLaunching.value = null
    waitingForLogin.value = null  // 清除等待状态
    // 继续启动下一个
    setTimeout(() => processLaunchQueue(), 2000)
  }
}

async function launchAll() {
  const ids = accounts.value
    .filter((a) => a.encrypted_token)
    .map((a) => a.id)

  if (!ids.length) {
    batchMsg.value = '没有已配置 Token 的账号'
    setTimeout(() => (batchMsg.value = ''), 3000)
    return
  }

  // 检查游戏路径是否已设置
  try {
    const settings: any = await invoke('get_settings')
    if (!settings.game_path || settings.game_path.trim() === '') {
      alert('⚠️ 请先设置 D2R.exe 所在目录！')
      tab.value = 'settings'
      return
    }
  } catch (e) {
    console.error('获取设置失败:', e)
    alert('获取设置失败，请稍后重试')
    return
  }

  if (batchLaunching.value) {
    batchMsg.value = '已经在批量启动中...'
    return
  }

  // 初始化队列
  launchQueue.value = [...ids]
  batchLaunching.value = true
  batchMsg.value = `准备启动 ${ids.length} 个账号...`
  
  // 开始处理队列
  processLaunchQueue()
}

const readyCount = () => accounts.value.filter((a) => a.encrypted_token).length
</script>

<template>
  <div class="app">
    <!-- 标题栏 -->
    <header class="topbar">
      <div class="topbar-title">
        <img src="/tauri.svg" class="topbar-icon" alt="" />
        <span>D2R CN 多开启动器</span>
      </div>
      <nav class="tabs">
        <button
          class="tab-btn"
          :class="{ active: tab === 'accounts' }"
          @click="tab = 'accounts'"
        >账号管理</button>
        <button
          class="tab-btn"
          :class="{ active: tab === 'settings' }"
          @click="tab = 'settings'"
        >⚙️ 设置</button>
      </nav>
    </header>

    <!-- 账号列表页 -->
    <main v-if="tab === 'accounts'" class="main-content">
      <div class="list-header">
        <div class="list-meta">
          共 {{ accounts.length }} 个账号，
          <span class="ready">{{ readyCount() }} 个已配置 Token</span>
          <span v-if="runningCount > 0" class="running-badge">🎮 {{ runningCount }} 个正在运行</span>
        </div>
        <div class="list-actions">
          <button class="btn btn-secondary" @click="openAddModal">+ 添加账号</button>
          <button
            class="btn btn-primary"
            :disabled="batchLaunching || readyCount() === 0"
            @click="launchAll"
          >
            {{ batchLaunching ? '启动中...' : '🚀 全部启动' }}
          </button>
        </div>
      </div>

      <div v-if="batchMsg" class="batch-msg">{{ batchMsg }}</div>

      <div v-if="accounts.length === 0" class="empty">
        <p>暂无账号，点击「添加账号」开始</p>
      </div>

      <AccountCard
        v-for="account in accounts"
        :key="account.id"
        :account="account"
        :running-game="runningGames.get(account.id)"
        @refresh="loadAccounts"
        @edit="openEditModal"
        @delete="handleDelete"
        @launch="handleLaunchAccount"
      />
    </main>

    <!-- 设置页 -->
    <main v-else-if="tab === 'settings'" class="main-content">
      <SettingsPanel />
    </main>

    <!-- 添加/编辑弹窗 -->
    <AccountModal
      :visible="showModal"
      :initial="editingAccount"
      @close="showModal = false"
      @save="handleSave"
    />

    <!-- 等待登录完成遮罩层 -->
    <transition name="fade">
      <div v-if="waitingForLogin" class="login-overlay">
        <div class="login-overlay-content">
          <div class="spinner"></div>
          <div class="login-text">
            <div class="login-title">🔐 正在等待登录完成</div>
            <div class="login-account">{{ waitingForLogin.accountName }}</div>
            <div class="login-hint">请在游戏中完成登录并进入角色选择界面</div>
            <div class="login-warning">⚠️ 在此期间请勿关闭游戏或启动其他账号</div>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  background: #0d1117;
  color: #e2e8f0;
  font-family: 'Segoe UI', system-ui, sans-serif;
  font-size: 14px;
  height: 100vh;
  overflow: hidden;
  user-select: none;
  -webkit-user-select: none;
  -moz-user-select: none;
  -ms-user-select: none;
}

input, textarea {
  user-select: text;
  -webkit-user-select: text;
  -moz-user-select: text;
  -ms-user-select: text;
}

#app { height: 100vh; display: flex; flex-direction: column; }

.app { display: flex; flex-direction: column; height: 100vh; }

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  height: 52px;
  background: #161b2e;
  border-bottom: 1px solid #1e2540;
  flex-shrink: 0;
}
.topbar-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 16px;
  font-weight: 700;
  color: #f1f5f9;
}
.topbar-icon {
  width: 22px;
  height: 22px;
  filter: drop-shadow(0 0 6px #3b82f6aa);
}
.tabs { display: flex; gap: 4px; }
.tab-btn {
  padding: 6px 16px;
  background: transparent;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  color: #94a3b8;
  font-size: 13px;
  font-weight: 500;
  transition: background 0.15s, color 0.15s;
}
.tab-btn:hover { background: #1e2a45; color: #e2e8f0; }
.tab-btn.active { background: #1e3a6e; color: #60a5fa; }

.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.list-meta { 
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px; 
  color: #64748b; 
}
.list-meta .ready { color: #86efac; }
.list-meta .running-badge {
  padding: 4px 10px;
  border-radius: 12px;
  background: rgba(74, 222, 128, 0.15);
  color: #4ade80;
  font-weight: 500;
  border: 1px solid rgba(74, 222, 128, 0.3);
}
.list-actions { display: flex; gap: 8px; }

.batch-msg {
  background: #1e2030;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 8px 14px;
  margin-bottom: 12px;
  font-size: 13px;
  color: #cbd5e1;
}

.empty {
  text-align: center;
  padding: 60px 0;
  color: #475569;
  font-size: 15px;
}

.btn {
  padding: 7px 16px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
}
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-primary   { background: #3b82f6; color: #fff; }
.btn-primary:not(:disabled):hover { background: #2563eb; }
.btn-secondary { background: #1e2a3a; color: #94a3b8; border: 1px solid #2d3050; }
.btn-secondary:hover { background: #253348; color: #e2e8f0; }

/* 滚动条 */
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: #2d3050; border-radius: 3px; }

/* 等待登录遮罩层 */
.login-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
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
  gap: 24px;
  padding: 48px 64px;
  background: linear-gradient(135deg, rgba(30, 41, 59, 0.95), rgba(15, 23, 42, 0.95));
  border-radius: 16px;
  border: 1px solid rgba(59, 130, 246, 0.3);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  animation: slideIn 0.3s ease-out;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(-20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.spinner {
  width: 64px;
  height: 64px;
  border: 4px solid rgba(59, 130, 246, 0.2);
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.login-text {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
  text-align: center;
}

.login-title {
  font-size: 20px;
  font-weight: 700;
  color: #f1f5f9;
  letter-spacing: -0.02em;
}

.login-account {
  font-size: 28px;
  font-weight: 800;
  color: #60a5fa;
  text-shadow: 0 0 16px rgba(96, 165, 250, 0.4);
  letter-spacing: -0.03em;
}

.login-hint {
  font-size: 14px;
  color: #94a3b8;
  margin-top: 8px;
}

.login-warning {
  font-size: 13px;
  color: #fbbf24;
  background: rgba(251, 191, 36, 0.1);
  padding: 8px 16px;
  border-radius: 6px;
  border: 1px solid rgba(251, 191, 36, 0.2);
  margin-top: 8px;
}

/* 淡入淡出动画 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.3s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>


<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

</style>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

</style>