<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { Settings, Account } from '../types'

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

// 批量窗口布局
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
      filters: [{
        name: 'D2R.exe',
        extensions: ['exe']
      }]
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
    selectedIds.value = accounts.value.map(a => a.id)
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

const emit = defineEmits<{
  'accounts-updated': []
}>()

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
    // 重新加载账号数据
    accounts.value = await invoke('get_accounts')
    // 通知父组件刷新账号列表
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
</script>

<template>
  <div class="settings-panel">
    <h2>⚙️ 设置</h2>

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
          <button class="btn btn-ghost" @click="clearPath" type="button">✕ 清空</button>
        </div>
        <p class="hint">程序会在此目录下查找 D2R.exe 并启动</p>
        <p class="hint success">✅ Handle64.exe 已内置到启动器中，无需单独配置</p>
      </div>
    </div>

    <div class="section">
      <h4>批量启动设置</h4>
      <div class="field">
        <label>
          <input type="checkbox" v-model="settings.wait_for_login" />
          等待登录完成后再启动下一个账号（推荐）
        </label>
        <p class="hint">
          启用后，批量启动时会监控注册表 Token 变化，确认前一个账号到达角色选择界面后再启动下一个。
          这样可以避免 Token 冲突导致的登录失败。
        </p>
      </div>
      
      <div class="field" v-if="settings.wait_for_login">
        <label>登录检测超时时间（秒）</label>
        <input v-model.number="settings.login_timeout_secs" type="number" min="30" max="180" />
        <p class="hint">如果超过此时间还未检测到登录完成，将自动继续启动下一个账号</p>
      </div>
      
      <div class="field" v-if="!settings.wait_for_login">
        <label>批量启动间隔（秒）</label>
        <input v-model.number="settings.launch_delay_secs" type="number" min="1" max="60" />
        <p class="hint">批量启动多个账号时，每个账号之间的等待时间</p>
      </div>
    </div>

    <div class="section">
      <h4>窗口设置</h4>
      <div class="field">
        <label>
          <input type="checkbox" v-model="settings.rename_window" />
          修改游戏窗口标题为账号名称（方便识别）
        </label>
        <p class="hint">
          启用后，游戏窗口标题将改为账号名称，便于区分多开窗口。
          不启用也不影响游戏监控功能（基于进程 PID 跟踪）。
        </p>
      </div>
    </div>

    <div class="section">
      <h4>📐 批量修改账号窗口布局</h4>
      <p class="hint" style="margin-bottom: 12px;">选择账号后，点击布局按钮自动分配不同窗口位置</p>

      <div v-if="batchMsg" class="batch-msg">{{ batchMsg }}</div>

      <div class="field">
        <label>选择账号 <button class="btn btn-ghost btn-xs" @click="toggleSelectAll">{{ selectedIds.length === accounts.length ? '取消全选' : '全选' }}</button></label>
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
        <button class="btn btn-danger" @click="resetBatchLayout">🗑️ 清空所选账号的窗口设置</button>
      </div>
    </div>

    <button class="btn btn-primary" :disabled="saving" @click="save">
      {{ saving ? '保存中...' : saved ? '✅ 已保存' : '保存设置' }}
    </button>
  </div>
</template>

<style scoped>
.settings-panel {
  padding: 24px;
  max-width: 680px;
}
h2 {
  margin: 0 0 24px;
  font-size: 20px;
  color: #e2e8f0;
}
.section {
  margin-bottom: 24px;
}
h4 {
  margin: 0 0 12px;
  font-size: 14px;
  color: #94a3b8;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid #1e2030;
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
  display: flex;
  align-items: center;
  gap: 8px;
}
input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
}
input[type="text"],
input[type="number"] {
  background: #0f1220;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 8px 12px;
  color: #e2e8f0;
  font-size: 14px;
  outline: none;
}
input[type="text"]:focus,
input[type="number"]:focus {
  border-color: #3b82f6;
}
.hint {
  font-size: 12px;
  color: #64748b;
  margin: 0;
}
.hint.success {
  color: #10b981;
  font-weight: 500;
}
.input-with-buttons {
  display: flex;
  gap: 8px;
  align-items: center;
}
.input-with-buttons input {
  flex: 1;
}
.input-with-buttons .btn {
  padding: 8px 14px;
  white-space: nowrap;
}
a { color: #60a5fa; }
.btn {
  padding: 9px 24px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
}
.btn-primary { background: #3b82f6; color: #fff; }
.btn-primary:hover { background: #2563eb; }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary { background: #374151; color: #e2e8f0; }
.btn-secondary:hover { background: #4b5563; }
.btn-ghost { background: transparent; color: #94a3b8; border: 1px solid #374151; }
.btn-ghost:hover { color: #e2e8f0; border-color: #6b7280; }
.btn-xs { padding: 2px 8px; font-size: 11px; }

.account-select-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  background: #0f1220;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 8px 10px;
  max-height: 120px;
  overflow-y: auto;
}
.account-check {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 13px;
  color: #cbd5e1;
  cursor: pointer;
}
.account-check input { cursor: pointer; }
.no-accounts { font-size: 12px; color: #64748b; }

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
  padding: 12px 8px;
  min-height: 68px;
  gap: 4px;
}
.layout-icon {
  font-size: 16px;
  line-height: 1.2;
  white-space: pre;
  letter-spacing: 2px;
}
.layout-label {
  font-size: 12px;
  color: #94a3b8;
}

.batch-msg {
  font-size: 13px;
  color: #cbd5e1;
  margin-bottom: 10px;
}
.reset-row {
  margin-top: 10px;
}
.btn-danger { background: transparent; color: #f87171; border: 1px solid #7f1d1d; }
.btn-danger:hover { background: rgba(220, 38, 38, 0.2); border-color: #ef4444; }
</style>
