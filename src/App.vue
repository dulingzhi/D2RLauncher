<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getVersion } from '@tauri-apps/api/app'
import { save as saveFileDialog } from '@tauri-apps/plugin-dialog'
import AccountCard from './components/AccountCard.vue'
import AccountModal from './components/AccountModal.vue'
import SettingsModal from './components/SettingsModal.vue'
import ToastContainer from './components/ToastContainer.vue'
import ConfirmDialog from './components/ConfirmDialog.vue'
import FlavorSelectDialog from './components/FlavorSelectDialog.vue'
import BackupPasswordDialog from './components/BackupPasswordDialog.vue'
import { useToast } from './composables/useToast'
import type { Account } from './types'
import { gameInfo } from './types'

const { success, error: toastError, warning, info, update: updateToast } = useToast()

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
/** 分支选择弹窗状态（检测到多个已安装分支时触发） */
const flavorDialog = ref<{ account: Account; flavors: string[] } | null>(null)

const confirmRef = ref<InstanceType<typeof ConfirmDialog> | null>(null)
const backupDialog = ref<InstanceType<typeof BackupPasswordDialog> | null>(null)

/* ========== 列表编辑模式：多选 + 批量删除/导出 + 拖动排序 ========== */
const editMode = ref(false)
const selectedIds = ref<Set<string>>(new Set())
/** 拖动排序：被拖账号 id / 插入位置（在当前数组中「插到此索引之前」） */
const dragId = ref<string | null>(null)
const dropBeforeIndex = ref(-1)
const mainRef = ref<HTMLElement | null>(null)

const selectedCount = computed(() => selectedIds.value.size)
const allSelected = computed(
  () => accounts.value.length > 0 && accounts.value.every((a) => selectedIds.value.has(a.id)),
)

function toggleEditMode() {
  editMode.value = !editMode.value
  selectedIds.value = new Set()
  dragId.value = null
  dropBeforeIndex.value = -1
}

function toggleSelect(id: string) {
  const next = new Set(selectedIds.value)
  if (next.has(id)) next.delete(id)
  else next.add(id)
  selectedIds.value = next
}

function toggleSelectAll() {
  if (allSelected.value) {
    selectedIds.value = new Set()
  } else {
    selectedIds.value = new Set(accounts.value.map((a) => a.id))
  }
}

/* ---- 指针拖动排序（WebView2 下 HTML5 DnD 光标/落点不可靠，改用 pointer events） ---- */

function onHandleDown(index: number, e: PointerEvent) {
  if (!editMode.value) return
  e.preventDefault()
  dragId.value = accounts.value[index]?.id ?? null
  if (dragId.value === null) return
  dropBeforeIndex.value = index
  window.addEventListener('pointermove', onDragMove)
  window.addEventListener('pointerup', onDragEnd, { once: true })
}

function onDragMove(e: PointerEvent) {
  const main = mainRef.value
  if (!main || dragId.value === null) return

  // 靠近列表视口边缘时自动滚动
  const r = main.getBoundingClientRect()
  if (e.clientY < r.top + 24) main.scrollTop -= 10
  else if (e.clientY > r.bottom - 24) main.scrollTop += 10

  const wraps = Array.from(main.querySelectorAll<HTMLElement>('.card-wrap'))
  for (let i = 0; i < wraps.length; i++) {
    const rect = wraps[i].getBoundingClientRect()
    if (e.clientY >= rect.top && e.clientY <= rect.bottom) {
      const mid = rect.top + rect.height / 2
      dropBeforeIndex.value = e.clientY < mid ? i : i + 1
      return
    }
  }
  // 指针在卡片区外：偏上放最前，偏下放最后
  dropBeforeIndex.value =
    wraps.length > 0 && e.clientY < wraps[0].getBoundingClientRect().top ? 0 : wraps.length
}

async function onDragEnd() {
  window.removeEventListener('pointermove', onDragMove)
  const id = dragId.value
  const to = dropBeforeIndex.value
  dragId.value = null
  dropBeforeIndex.value = -1
  if (id === null || to < 0) return

  const list = [...accounts.value]
  const from = list.findIndex((a) => a.id === id)
  if (from < 0) return
  // 目标位置指向被拖元素自身或其紧后位置 → 顺序不变
  if (to === from || to === from + 1) return

  const [moved] = list.splice(from, 1)
  list.splice(to > from ? to - 1 : to, 0, moved)
  accounts.value = list

  try {
    await invoke('reorder_accounts', { ids: list.map((a) => a.id) })
  } catch (e) {
    toastError(`保存顺序失败: ${e}`)
  }
}

async function handleBatchDelete() {
  if (!selectedCount.value) return
  const ok = await confirmRef.value?.confirm({
    title: '删除账号',
    message: `确认删除选中的 ${selectedCount.value} 个账号？此操作不可撤销。`,
    confirmText: '删除',
    danger: true,
  })
  if (!ok) return
  try {
    const removed = await invoke<number>('delete_accounts', { ids: [...selectedIds.value] })
    success(`已删除 ${removed} 个账号`)
    selectedIds.value = new Set()
    await loadAccounts()
  } catch (e) {
    toastError(`删除失败: ${e}`)
  }
}

async function handleBatchExport() {
  if (!selectedCount.value) return
  const result = await backupDialog.value?.open('export')
  if (!result) return

  try {
    const defaultName = `d2r-backup-${new Date().toISOString().slice(0, 10)}.json`
    const path = await saveFileDialog({
      title: '导出账号备份',
      defaultPath: defaultName,
      filters: [{ name: '加密备份文件', extensions: ['json'] }],
    })
    if (!path) return

    const count = await invoke<number>('export_accounts', {
      path,
      password: result.password,
      accountIds: [...selectedIds.value],
    })
    success(`✅ 已导出 ${count} 个账号到加密备份`)
  } catch (e) {
    toastError(`导出失败: ${e}`)
  }
}

let unlistenGameStatus: UnlistenFn | null = null
let unlistenLoginComplete: UnlistenFn | null = null

/** 异步检查更新，不阻塞主流程 */
async function checkForUpdate() {
  try {
    const currentVersion = await getVersion()
    console.log('[updater] currentVersion:', currentVersion)
    const updateInfo = await invoke<{
      version: string
      notes: string | null
      pub_date: string | null
      download_url: string
    } | null>('check_update')
    console.log('[updater] updateInfo:', updateInfo)
    if (!updateInfo) return

    const ok = await confirmRef.value?.confirm({
      title: '发现新版本',
      message: `当前版本 ${currentVersion}，新版本 ${updateInfo.version}\n\n更新内容：\n${updateInfo.notes || ''}\n\n是否立即下载并安装？`,
      confirmText: '下载更新',
    })
    if (!ok) return

    const progressToast = info('正在下载更新...', 0)

    const unlistenProgress = await listen<{ downloaded: number; total: number }>(
      'update-download-progress',
      (event) => {
        const { downloaded, total } = event.payload
        const percent = total > 0 ? ((downloaded / total) * 100).toFixed(0) : '0'
        updateToast(progressToast.id, `下载进度: ${percent}%`)
      },
    )

    try {
      await invoke('perform_self_update', { downloadUrl: updateInfo.download_url })
    } catch (e) {
      toastError(`更新失败: ${e}`)
    } finally {
      unlistenProgress()
    }
  } catch {
    // 静默处理更新检查失败
  }
}

async function loadAccounts() {
  accounts.value = await invoke('get_accounts')
}

const runningCount = computed(() => runningGames.value.size)

onMounted(async () => {
  // 清理上次更新残留的临时文件
  invoke('cleanup_update').catch(() => {})

  // 检查更新（异步，不阻塞启动）
  if (import.meta.env.PROD) {
    checkForUpdate()
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
        game: data.game,
        flavor: data.flavor,
        region: data.region,
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

/** 取指定游戏的安装目录（与后端 game_path_for 一致：优先 game_paths，osic 回退旧字段） */
function gamePathFor(settings: any, game: string): string {
  const fromMap = settings.game_paths?.[game]
  if (fromMap && fromMap.trim() !== '') return fromMap
  if (game === 'osic') return settings.game_path ?? ''
  return ''
}

/** 实际执行启动（flavor 覆盖账号记忆值，用于单分支直启/弹窗选择结果） */
async function doLaunch(account: Account, flavorOverride: string | null) {
  const settings: any = await invoke('get_settings')

  if (settings.wait_for_login) {
    currentLaunching.value = account.id
    waitingForLogin.value = { accountId: account.id, accountName: account.label }
  }

  await invoke('launch_account', {
    accountId: account.id,
    flavor: flavorOverride,
  })

  if (!settings.wait_for_login) {
    currentLaunching.value = null
    waitingForLogin.value = null
  }
}

async function handleLaunchAccount(account: Account) {
  try {
    const settings: any = await invoke('get_settings')
    const game = account.game || 'osic'

    if (!gamePathFor(settings, game).trim()) {
      warning(`请先在设置中配置「${gameInfo(game).label}」的安装目录`)
      showSettings.value = true
      return
    }

    // 多分支游戏：检测已装分支。装了多个才弹窗；只有一个直接用已装分支启动
    const installed: string[] = await invoke('get_installed_flavors', { game })
    if (installed.length >= 2) {
      flavorDialog.value = { account, flavors: installed }
      return
    }
    await doLaunch(account, installed.length === 1 ? installed[0] : null)
  } catch (e) {
    toastError(`启动失败: ${e}`)
    currentLaunching.value = null
    waitingForLogin.value = null
  }
}

/** 弹窗确认分支：记住本次选择后启动 */
async function handleFlavorConfirm(flavor: string) {
  const dlg = flavorDialog.value
  if (!dlg) return
  flavorDialog.value = null

  try {
    // 记住本次选择，作为下次弹窗预选项与批量启动的记忆值
    await invoke('update_account', { account: { ...dlg.account, flavor } })
    await loadAccounts()
  } catch {
    // 记忆失败不阻断启动
  }

  try {
    await doLaunch(dlg.account, flavor)
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
    const game = account.game || 'osic'
    if (!gamePathFor(settings, game).trim()) {
      throw new Error(`请先在设置中配置「${gameInfo(game).label}」的安装目录`)
    }
    await invoke('launch_account', { accountId })

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
    // 逐账号校验安装目录（可能混合多个游戏），缺失即中止并提示
    const settings: any = await invoke('get_settings')
    const missing = accounts.value.find(
      (a) => a.encrypted_token && !gamePathFor(settings, a.game || 'osic').trim(),
    )
    if (missing) {
      const game = missing.game || 'osic'
      warning(`账号「${missing.label}」未配置「${gameInfo(game).label}」的安装目录`)
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
    <main class="main-content" ref="mainRef">
      <!-- 操作栏 -->
      <div class="toolbar">
        <div class="toolbar-left">
          <span v-if="editMode" class="toolbar-meta">
            已选 <strong>{{ selectedCount }}</strong> / 共 {{ accounts.length }} 个，
            <span class="text-blue">按住 ⠿ 手柄可拖动排序</span>
          </span>
          <span v-else class="toolbar-meta">
            共 <strong>{{ accounts.length }}</strong> 个账号，
            <span class="text-green">{{ readyCount() }} 个已配置</span>
          </span>
        </div>
        <div class="toolbar-right">
          <template v-if="editMode">
            <button class="btn btn-secondary" @click="toggleSelectAll">
              {{ allSelected ? '取消全选' : '全选' }}
            </button>
            <button
              class="btn btn-danger"
              :disabled="selectedCount === 0"
              @click="handleBatchDelete"
            >
              🗑️ 删除{{ selectedCount > 0 ? ` (${selectedCount})` : '' }}
            </button>
            <button
              class="btn btn-secondary"
              :disabled="selectedCount === 0"
              @click="handleBatchExport"
            >
              📤 导出{{ selectedCount > 0 ? ` (${selectedCount})` : '' }}
            </button>
            <button class="btn btn-primary" @click="toggleEditMode">✔️ 完成</button>
          </template>
          <template v-else>
            <button class="btn btn-secondary" @click="openAddModal">+ 添加</button>
            <button class="btn btn-secondary" @click="toggleEditMode">✏️ 管理</button>
            <button
              class="btn btn-primary"
              :disabled="batchLaunching || readyCount() === 0"
              @click="launchAll"
            >
              {{ batchLaunching ? '启动中...' : '🚀 全部启动' }}
            </button>
          </template>
        </div>
      </div>

      <!-- 空状态 -->
      <div v-if="accounts.length === 0" class="empty-state">
        <div class="empty-icon">📂</div>
        <p>暂无账号</p>
        <p class="empty-hint">点击「+ 添加」开始</p>
      </div>

      <!-- 账号卡片列表（编辑模式下按住手柄拖动排序） -->
      <div
        v-for="(account, index) in accounts"
        :key="account.id"
        class="card-wrap"
        :class="{
          dragging: dragId === account.id,
          'insert-before': dropBeforeIndex === index && dragId !== account.id,
          'insert-after':
            dropBeforeIndex === index + 1 &&
            index === accounts.length - 1 &&
            dragId !== account.id,
        }"
      >
        <AccountCard
          :account="account"
          :running-game="runningGames.get(account.id)"
          :edit-mode="editMode"
          :selected="selectedIds.has(account.id)"
          @refresh="loadAccounts"
          @edit="openEditModal"
          @delete="handleDelete"
          @launch="handleLaunchAccount"
          @kill="handleKillProcess"
          @select="toggleSelect(account.id)"
          @drag-handle-down="onHandleDown(index, $event)"
        />
      </div>
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

    <!-- 分支选择弹窗 -->
    <FlavorSelectDialog
      :visible="!!flavorDialog"
      :account-name="flavorDialog?.account.label ?? ''"
      :flavors="flavorDialog?.flavors ?? []"
      :default-flavor="flavorDialog?.account.flavor"
      @confirm="handleFlavorConfirm"
      @cancel="flavorDialog = null"
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

    <!-- 批量导出密码弹窗 -->
    <BackupPasswordDialog ref="backupDialog" />
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
.text-blue { color: #60a5fa; }

.toolbar-right { display: flex; gap: 6px; }

.btn-danger {
  background: transparent;
  color: #f87171;
  border: 1px solid #7f1d1d;
}

.btn-danger:not(:disabled):hover {
  background: rgba(220, 38, 38, 0.15);
  border-color: #ef4444;
}

.btn-danger:disabled { opacity: 0.45; cursor: not-allowed; }

/* ========== 卡片拖拽排序 ========== */
.card-wrap { position: relative; }

.card-wrap.dragging { opacity: 0.35; }

.card-wrap.insert-before::before,
.card-wrap.insert-after::after {
  content: '';
  position: absolute;
  left: 0;
  right: 0;
  height: 2px;
  background: #3b82f6;
  border-radius: 1px;
  pointer-events: none;
}

.card-wrap.insert-before::before { top: -4px; }
.card-wrap.insert-after::after { bottom: -4px; }

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
