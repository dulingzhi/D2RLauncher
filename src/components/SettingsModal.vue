<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open, save as saveFileDialog } from '@tauri-apps/plugin-dialog'
import type { Settings, Account, ImportResult } from '../types'
import { useToast } from '../composables/useToast'
import BackupPasswordDialog from './BackupPasswordDialog.vue'

defineProps<{ visible: boolean }>()
const emit = defineEmits<{
  close: []
  'accounts-updated': []
}>()

const settings = ref<Settings>({
  game_path: '',
  handle_path: '',
  launch_delay_secs: 5,
  wait_for_login: true,
  login_timeout_secs: 60,
  rename_window: true,
})
const saving = ref(false)
const saved = ref(false)

const accounts = ref<Account[]>([])
const selectedIds = ref<string[]>([])
const batchMsg = ref('')

const layouts = [
  { key: 'horizontal', label: '横排', icon: '▭▭▭' },
  { key: 'vertical', label: '竖排', icon: '▭\n▭\n▭' },
  { key: 'grid2x2', label: '2×2 网格', icon: '▭▭\n▭▭' },
  { key: 'grid2x3', label: '2×3 网格', icon: '▭▭▭\n▭▭▭' },
] as const

onMounted(async () => {
  try {
    settings.value = await invoke('get_settings')
  } catch (e) {
    console.error(e)
  }
  try {
    accounts.value = await invoke('get_accounts')
  } catch (e) {
    console.error(e)
  }
})

async function save() {
  saving.value = true
  try {
    await invoke('save_settings', { settings: settings.value })
    saved.value = true
    setTimeout(() => (saved.value = false), 2000)
  } catch (e) {
    console.error(e)
  } finally {
    saving.value = false
  }
}

async function browsePath() {
  try {
    const selected = await open({
      title: '选择 D2R.exe',
      multiple: false,
      directory: false,
      filters: [{ name: 'D2R.exe', extensions: ['exe'] }],
    })

    if (selected && typeof selected === 'string') {
      const path = selected.replace(/\\/g, '/')
      const lastSlash = path.lastIndexOf('/')
      if (lastSlash > 0) {
        settings.value.game_path = path.substring(0, lastSlash)
      }
    }
  } catch (e) {
    console.error('选择文件失败:', e)
  }
}

function clearPath() {
  settings.value.game_path = ''
}

function toggleSelectAll() {
  if (selectedIds.value.length === accounts.value.length) {
    selectedIds.value = []
  } else {
    selectedIds.value = accounts.value.map((a) => a.id)
  }
}

function toggleAccount(id: string) {
  const idx = selectedIds.value.indexOf(id)
  if (idx >= 0) {
    selectedIds.value.splice(idx, 1)
  } else {
    selectedIds.value.push(id)
  }
}

async function applyBatchLayout(layout: string) {
  if (selectedIds.value.length === 0) {
    batchMsg.value = '⚠️ 请至少选择一个账号'
    setTimeout(() => (batchMsg.value = ''), 3000)
    return
  }
  try {
    const count = await invoke('batch_update_account_windows', {
      accountIds: selectedIds.value,
      layout,
    })
    batchMsg.value = `✅ 已更新 ${count} 个账号的窗口布局`
    accounts.value = await invoke('get_accounts')
    emit('accounts-updated')
    setTimeout(() => (batchMsg.value = ''), 3000)
  } catch (e: any) {
    batchMsg.value = `❌ ${e}`
    setTimeout(() => (batchMsg.value = ''), 3000)
  }
}

async function resetBatchLayout() {
  if (selectedIds.value.length === 0) {
    batchMsg.value = '⚠️ 请至少选择一个账号'
    setTimeout(() => (batchMsg.value = ''), 3000)
    return
  }
  try {
    const count = await invoke('reset_account_windows', {
      accountIds: selectedIds.value,
    })
    batchMsg.value = `✅ 已清空 ${count} 个账号的窗口设置`
    accounts.value = await invoke('get_accounts')
    emit('accounts-updated')
    setTimeout(() => (batchMsg.value = ''), 3000)
  } catch (e: any) {
    batchMsg.value = `❌ ${e}`
    setTimeout(() => (batchMsg.value = ''), 3000)
  }
}

const toast = useToast()
const backupDialog = ref<InstanceType<typeof BackupPasswordDialog> | null>(null)

async function handleExport() {
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
    })
    toast.success(`✅ 已导出 ${count} 个账号到加密备份`)
  } catch (e: any) {
    toast.error(`❌ 导出失败: ${e}`)
  }
}

async function handleImport() {
  try {
    const path = await open({
      title: '选择备份文件',
      multiple: false,
      directory: false,
      filters: [{ name: '加密备份文件', extensions: ['json'] }],
    })
    if (!path || typeof path !== 'string') return

    const result = await backupDialog.value?.open('import')
    if (!result) return

    const stats = await invoke<ImportResult>('import_accounts', {
      path,
      password: result.password,
      mode: result.mode,
    })
    const parts = [`新增 ${stats.imported}`]
    if (stats.updated > 0) parts.push(`更新 ${stats.updated}`)
    if (stats.skipped > 0) parts.push(`跳过 ${stats.skipped}`)
    toast.success(`✅ 导入完成：${parts.join('，')}`)

    accounts.value = await invoke('get_accounts')
    emit('accounts-updated')
  } catch (e: any) {
    toast.error(`❌ 导入失败: ${e}`)
  }
}
</script>

<template>
  <Teleport to="body">
    <BackupPasswordDialog ref="backupDialog" />
    <Transition name="settings-modal">
      <div v-if="visible" class="settings-modal-overlay" @click.self="emit('close')">
        <div class="settings-modal">
          <div class="settings-modal-header">
            <h2>⚙️ 设置</h2>
            <button class="close-btn" @click="emit('close')" title="关闭">✕</button>
          </div>

          <div class="settings-modal-body">
            <!-- 游戏路径 -->
            <div class="section">
              <h4>游戏路径</h4>
              <div class="field">
                <label>D2R.exe 所在目录</label>
                <div class="input-with-buttons">
                  <input
                    v-model="settings.game_path"
                    placeholder="如：C:\Program Files (x86)\Battle.net\Games\Diablo II Resurrected"
                  />
                  <button class="btn btn-secondary" @click="browsePath" type="button">📁 浏览</button>
                  <button class="btn btn-ghost" @click="clearPath" type="button">✕</button>
                </div>
                <p class="hint">程序会在此目录下查找 D2R.exe 并启动</p>
                <p class="hint success">✅ Handle64.exe 已内置，无需单独配置</p>
              </div>
            </div>

            <!-- 批量启动 -->
            <div class="section">
              <h4>批量启动</h4>
              <div class="field">
                <label class="toggle-label">
                  <input type="checkbox" v-model="settings.wait_for_login" />
                  <span>等待登录完成后再启动下一个（推荐）</span>
                </label>
                <p class="hint">
                  启用后，批量启动时监控注册表 Token 变化，确认前一个账号到达角色选择界面再启动下一个，避免 Token 冲突。
                </p>
              </div>

              <div class="field" v-if="settings.wait_for_login">
                <label>登录检测超时（秒）</label>
                <input v-model.number="settings.login_timeout_secs" type="number" min="30" max="180" />
                <p class="hint">超时后自动继续下一个</p>
              </div>

              <div class="field" v-if="!settings.wait_for_login">
                <label>启动间隔（秒）</label>
                <input v-model.number="settings.launch_delay_secs" type="number" min="1" max="60" />
                <p class="hint">批量启动时每个账号之间的延迟</p>
              </div>
            </div>

            <!-- 窗口设置 -->
            <div class="section">
              <h4>窗口</h4>
              <div class="field">
                <label class="toggle-label">
                  <input type="checkbox" v-model="settings.rename_window" />
                  <span>修改游戏窗口标题为账号名称</span>
                </label>
                <p class="hint">启用后便于区分多开窗口，不影响进程 PID 监控功能。</p>
              </div>
            </div>

            <!-- 批量窗口布局 -->
            <div class="section">
              <h4>📐 批量窗口布局</h4>

              <div v-if="batchMsg" class="batch-msg">{{ batchMsg }}</div>

              <div class="field">
                <div class="select-header">
                  <label>选择账号</label>
                  <button class="btn btn-ghost btn-xs" @click="toggleSelectAll">
                    {{ selectedIds.length === accounts.length ? '取消全选' : '全选' }}
                  </button>
                </div>
                <div class="account-select-list">
                  <label v-for="acc in accounts" :key="acc.id" class="account-check">
                    <input type="checkbox" :checked="selectedIds.includes(acc.id)" @change="toggleAccount(acc.id)" />
                    <span>{{ acc.label }}</span>
                  </label>
                  <span v-if="accounts.length === 0" class="no-accounts">暂无账号</span>
                </div>
              </div>

              <div class="layout-grid">
                <button
                  v-for="preset in layouts"
                  :key="preset.key"
                  class="btn btn-secondary layout-btn"
                  @click="applyBatchLayout(preset.key)"
                  :title="preset.label"
                >
                  <span class="layout-icon">{{ preset.icon }}</span>
                  <span class="layout-label">{{ preset.label }}</span>
                </button>
              </div>

              <div class="reset-row">
                <button class="btn btn-danger" @click="resetBatchLayout">🗑️ 清空所选窗口设置</button>
              </div>
            </div>

            <!-- 数据管理 -->
            <div class="section">
              <h4>💾 数据管理</h4>
              <p class="hint" style="margin-bottom: 10px">
                导出全部账号（含认证 Token 和窗口设置）为密码加密的备份文件，可跨机器迁移；导入时需输入相同密码解密。
              </p>
              <div class="data-actions">
                <button class="btn btn-secondary" @click="handleExport">📤 导出账号数据</button>
                <button class="btn btn-secondary" @click="handleImport">📥 导入账号数据</button>
              </div>
            </div>
          </div>

          <div class="settings-modal-footer">
            <button class="btn btn-primary btn-save" :disabled="saving" @click="save">
              {{ saving ? '保存中...' : saved ? '✅ 已保存' : '保存设置' }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.settings-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
}

.settings-modal {
  background: #161b2e;
  border: 1px solid #2d3050;
  border-radius: 14px;
  width: 620px;
  max-width: 95vw;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.settings-modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 24px;
  border-bottom: 1px solid #2d3050;
  flex-shrink: 0;
}

.settings-modal-header h2 {
  margin: 0;
  font-size: 17px;
  font-weight: 600;
  color: #e2e8f0;
}

.close-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  font-size: 20px;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  transition: all 0.15s;
  line-height: 1;
}

.close-btn:hover {
  background: rgba(148, 163, 184, 0.15);
  color: #e2e8f0;
}

.settings-modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

.settings-modal-footer {
  padding: 14px 24px;
  border-top: 1px solid #2d3050;
  display: flex;
  justify-content: flex-end;
  flex-shrink: 0;
}

.section {
  margin-bottom: 20px;
}

.section:last-child {
  margin-bottom: 0;
}

h4 {
  margin: 0 0 10px;
  font-size: 12px;
  color: #64748b;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  font-weight: 600;
  border-bottom: 1px solid #1e2540;
  padding-bottom: 6px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

label {
  font-size: 13px;
  color: #cbd5e1;
}

.toggle-label {
  display: flex !important;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}

.toggle-label span {
  flex: 1;
}

input[type="checkbox"] {
  width: 16px;
  height: 16px;
  cursor: pointer;
  accent-color: #3b82f6;
}

input[type="text"],
input[type="number"] {
  background: #0d1117;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 7px 12px;
  color: #e2e8f0;
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
}

input[type="text"]:focus,
input[type="number"]:focus {
  border-color: #3b82f6;
}

.hint {
  font-size: 11px;
  color: #475569;
  margin: 0;
  line-height: 1.5;
}

.hint.success {
  color: #10b981;
  font-weight: 500;
}

.input-with-buttons {
  display: flex;
  gap: 6px;
  align-items: center;
}

.input-with-buttons input {
  flex: 1;
}

.select-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.account-select-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 12px;
  background: #0d1117;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 8px 12px;
  max-height: 100px;
  overflow-y: auto;
}

.account-check {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 12px;
  color: #cbd5e1;
  cursor: pointer;
}

.no-accounts {
  font-size: 12px;
  color: #475569;
}

.layout-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-top: 8px;
}

.layout-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 10px 6px;
  min-height: 58px;
  gap: 3px;
}

.layout-icon {
  font-size: 14px;
  line-height: 1.2;
  white-space: pre;
  letter-spacing: 1px;
}

.layout-label {
  font-size: 11px;
  color: #94a3b8;
}

.batch-msg {
  font-size: 12px;
  color: #cbd5e1;
  margin-bottom: 8px;
}

.reset-row {
  margin-top: 10px;
}

.data-actions {
  display: flex;
  gap: 8px;
}

/* Buttons */
.btn {
  padding: 7px 14px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: all 0.15s;
  white-space: nowrap;
}

.btn-primary { background: #3b82f6; color: #fff; }
.btn-primary:hover:not(:disabled) { background: #2563eb; }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }

.btn-secondary { background: #1e2a3a; color: #94a3b8; border: 1px solid #2d3050; }
.btn-secondary:hover { background: #253348; color: #e2e8f0; }

.btn-ghost { background: transparent; color: #94a3b8; border: 1px solid #374151; }
.btn-ghost:hover { color: #e2e8f0; border-color: #6b7280; }

.btn-xs { padding: 2px 8px; font-size: 10px; }

.btn-save { padding: 9px 28px; font-size: 13px; }

.btn-danger { background: transparent; color: #f87171; border: 1px solid #7f1d1d; }
.btn-danger:hover { background: rgba(220, 38, 38, 0.15); border-color: #ef4444; }

.settings-modal-enter-active,
.settings-modal-leave-active { transition: all 0.2s ease; }

.settings-modal-enter-from,
.settings-modal-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-8px);
}
</style>
