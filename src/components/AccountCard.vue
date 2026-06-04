<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Account } from '../types'
import { tokenAge } from '../types'
import LoginDialog from './LoginDialog.vue'

interface GameInstance {
  account_id: string
  account_name: string
  region: string
  process_id: number
  window_title: string
  start_time: string
  total_seconds: number
}

const props = defineProps<{
  account: Account
  runningGame?: GameInstance
}>()
const emit = defineEmits<{
  refresh: []
  edit: [account: Account]
  delete: [id: string]
  launch: [account: Account]
}>()

const launching = ref(false)
const gettingToken = ref(false)
const statusMsg = ref('')
const showLoginDialog = ref(false)

async function handleTokenCaptured(token: string) {
  try {
    await invoke('save_token_for_account', {
      id: props.account.id,
      plainToken: token,
    })
    statusMsg.value = '✅ 登录认证获取成功'
    emit('refresh')
  } catch (e) {
    statusMsg.value = `❌ 保存登录认证失败: ${e}`
  } finally {
    showLoginDialog.value = false
    gettingToken.value = false
    setTimeout(() => (statusMsg.value = ''), 3000)
  }
}

function handleLoginCancel() {
  showLoginDialog.value = false
  gettingToken.value = false
  statusMsg.value = '已取消登录'
  setTimeout(() => (statusMsg.value = ''), 3000)
}

async function getToken() {
  gettingToken.value = true
  statusMsg.value = '正在打开登录窗口...'
  showLoginDialog.value = true
}

async function launch() {
  // 发送启动事件到 App.vue，由父组件统一处理遮罩层
  emit('launch', props.account)
}

const hasToken = computed(() => !!props.account.encrypted_token)
const tokenInfo = computed(() => tokenAge(props.account.token_set_at))
const isRunning = computed(() => !!props.runningGame)

const runningDuration = computed(() => {
  if (!props.runningGame) return ''
  const seconds = props.runningGame.total_seconds || 0
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60
  if (hours > 0) {
    return `${hours}小时${minutes}分钟`
  } else if (minutes > 0) {
    return `${minutes}分${secs}秒`
  } else {
    return `${secs}秒`
  }
})

async function killProcess() {
  if (!props.runningGame) return
  
  const confirmed = confirm(`确定要关闭游戏进程吗？\nPID: ${props.runningGame.process_id}`)
  if (!confirmed) return
  
  try {
    await invoke('kill_game_process', { processId: props.runningGame.process_id })
    statusMsg.value = '✅ 进程已关闭'
    emit('refresh')
  } catch (e) {
    statusMsg.value = `❌ 关闭失败: ${e}`
  } finally {
    setTimeout(() => (statusMsg.value = ''), 3000)
  }
}
</script>

<template>
  <div class="account-card" :class="{ 'has-token': hasToken, 'is-running': isRunning }">
    <div class="account-header">
      <div class="account-status-dot" :class="isRunning ? 'running' : (hasToken ? 'online' : 'offline')" />
      <div class="account-info">
        <span class="account-label">
          {{ account.label }}
          <span v-if="isRunning" class="running-badge">🎮 运行中</span>
        </span>
        <span class="account-email">{{ account.email }}</span>
      </div>
      <div class="account-token-info">
        <span :class="hasToken ? 'token-ok' : 'token-missing'">
          {{ hasToken ? `登录认证：${tokenInfo}` : '未获取登录认证' }}
        </span>
        <span v-if="isRunning" class="running-info">
          运行时长: {{ runningDuration }}
        </span>
        <span v-if="isRunning" class="running-info">
          PID: {{ runningGame?.process_id }}
        </span>
      </div>
    </div>

    <div v-if="statusMsg" class="status-msg">{{ statusMsg }}</div>

    <div class="account-actions">
      <button
        v-if="!isRunning"
        class="btn btn-primary"
        :disabled="!hasToken || launching"
        @click="launch"
      >
        {{ launching ? '启动中...' : '🚀 启动' }}
      </button>
      <button
        v-else
        class="btn btn-danger-solid"
        @click="killProcess"
      >
        ⛔ 关闭进程
      </button>
      <button
        class="btn btn-secondary"
        :disabled="gettingToken"
        @click="getToken"
      >
        {{ gettingToken ? '等待登录...' : hasToken ? '🔄 刷新登录认证' : '🔑 获取登录认证' }}
      </button>
      <button class="btn btn-ghost" @click="emit('edit', account)">编辑</button>
      <button class="btn btn-danger" @click="emit('delete', account.id)">删除</button>
    </div>
  </div>

  <LoginDialog
    :visible="showLoginDialog"
    :account-id="account.id"
    @token-captured="handleTokenCaptured"
    @cancel="handleLoginCancel"
  />
</template>

<style scoped>
.account-card {
  background: #1e2030;
  border: 1px solid #2d3050;
  border-radius: 8px;
  padding: 14px 16px;
  margin-bottom: 10px;
  transition: all 0.3s ease;
}
.account-card.has-token {
  border-color: #3d4f7c;
}
.account-card.is-running {
  background: #1e2f1e;
  border-color: #4ade80;
  box-shadow: 0 0 15px rgba(74, 222, 128, 0.2);
}
.account-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
}
.account-status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.account-status-dot.online  { background: #4ade80; }
.account-status-dot.offline { background: #6b7280; }
.account-status-dot.running { 
  background: #4ade80;
  box-shadow: 0 0 8px #4ade80;
  animation: pulse 2s infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.5; }
}
.account-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
}
.account-label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 15px;
  font-weight: 600;
  color: #f1f5f9;
}
.running-badge {
  font-size: 11px;
  padding: 2px 6px;
  border-radius: 8px;
  background: rgba(74, 222, 128, 0.2);
  color: #4ade80;
  font-weight: 500;
}
.account-email {
  font-size: 12px;
  color: #94a3b8;
}
.account-token-info {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 4px;
  font-size: 12px;
}
.running-info {
  color: #4ade80;
  font-family: 'Courier New', monospace;
  font-size: 11px;
  padding: 2px 6px;
  background: rgba(74, 222, 128, 0.1);
  border-radius: 4px;
}
.token-ok     { color: #86efac; }
.token-missing { color: #f87171; }
.status-msg {
  font-size: 13px;
  color: #cbd5e1;
  margin: 4px 0 8px 22px;
  min-height: 18px;
}
.account-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.btn {
  padding: 6px 14px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: opacity 0.15s, transform 0.1s;
}
.btn:active { transform: scale(0.97); }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-primary   { background: #3b82f6; color: #fff; }
.btn-primary:not(:disabled):hover { background: #2563eb; }
.btn-secondary { background: #374151; color: #e2e8f0; }
.btn-secondary:not(:disabled):hover { background: #4b5563; }
.btn-ghost     { background: transparent; color: #94a3b8; border: 1px solid #374151; }
.btn-ghost:hover { color: #e2e8f0; border-color: #6b7280; }
.btn-danger    { background: transparent; color: #f87171; border: 1px solid #374151; }
.btn-danger:hover { background: #7f1d1d33; border-color: #f87171; }
.btn-danger-solid { background: #dc2626; color: #fff; }
.btn-danger-solid:hover { background: #b91c1c; }
</style>
