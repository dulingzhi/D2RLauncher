<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Account } from '../types'
import { tokenAge, gameInfo, wowFlavorLabel } from '../types'
import LoginDialog from './LoginDialog.vue'
import { useToast } from '../composables/useToast'

const { success, error: toastError, info } = useToast()

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
  kill: [processId: number, accountName: string]
}>()

const launching = ref(false)
const gettingToken = ref(false)
const showLoginDialog = ref(false)

async function handleTokenCaptured(token: string) {
  try {
    await invoke('save_token_for_account', {
      id: props.account.id,
      plainToken: token,
    })
    success('登录认证获取成功')
    emit('refresh')
  } catch (e) {
    toastError(`保存登录认证失败: ${e}`)
  } finally {
    showLoginDialog.value = false
    gettingToken.value = false
  }
}

function handleLoginCancel() {
  showLoginDialog.value = false
  gettingToken.value = false
}

async function getToken() {
  gettingToken.value = true
  info('正在打开登录窗口...')
  showLoginDialog.value = true
}

async function launch() {
  emit('launch', props.account)
}

const hasToken = computed(() => !!props.account.encrypted_token)
const tokenInfo = computed(() => tokenAge(props.account.token_set_at))
const isRunning = computed(() => !!props.runningGame)
const game = computed(() => gameInfo(props.account.game))
const flavorText = computed(() =>
  props.account.game === 'wow' ? wowFlavorLabel(props.account.flavor) : '',
)

const runningDuration = computed(() => {
  if (!props.runningGame) return ''
  const seconds = props.runningGame.total_seconds || 0
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const secs = seconds % 60
  if (hours > 0) return `${hours}时${minutes}分`
  if (minutes > 0) return `${minutes}分${secs}秒`
  return `${secs}秒`
})

function killProcess() {
  emit('kill', props.runningGame!.process_id, props.account.label)
}
</script>

<template>
  <div class="account-card" :class="{ 'has-token': hasToken, 'is-running': isRunning }">
    <div class="account-row">
      <!-- 状态指示 + 名称 -->
      <div class="account-status-dot" :class="isRunning ? 'running' : (hasToken ? 'online' : 'offline')" />
      <div class="account-info">
        <span class="account-label">
          {{ account.label }}
          <span class="game-badge">{{ game.icon }} {{ game.short }}<template v-if="flavorText"> · {{ flavorText }}</template></span>
          <span v-if="isRunning" class="running-badge">运行中</span>
        </span>
        <span class="account-sub">
          <span :class="hasToken ? 'text-green' : 'text-red'">
            {{ hasToken ? `认证: ${tokenInfo}` : '未获取认证' }}
          </span>
          <span v-if="isRunning" class="running-time">{{ runningDuration }}</span>
        </span>
      </div>

      <!-- 操作按钮 -->
      <div class="account-actions">
        <button
          v-if="!isRunning"
          class="btn btn-primary btn-sm"
          :disabled="!hasToken || launching"
          @click="launch"
        >
          {{ launching ? '...' : '🚀 启动' }}
        </button>
        <button
          v-else
          class="btn btn-danger-solid btn-sm"
          @click="killProcess"
        >
          ⛔ 关闭
        </button>
        <button
          class="btn btn-secondary btn-sm"
          :disabled="gettingToken"
          @click="getToken"
        >
          {{ gettingToken ? '等待中...' : (hasToken ? '🔄 刷新' : '🔑 获取') }}
        </button>
        <button class="btn btn-ghost btn-sm" @click="emit('edit', account)">编辑</button>
        <button class="btn btn-danger btn-sm" @click="emit('delete', account.id)">删除</button>
      </div>
    </div>
  </div>

  <LoginDialog
    :visible="showLoginDialog"
    :account-id="account.id"
    :game="account.game || 'osic'"
    @token-captured="handleTokenCaptured"
    @cancel="handleLoginCancel"
  />
</template>

<style scoped>
.account-card {
  background: #1e2030;
  border: 1px solid #2d3050;
  border-radius: 8px;
  padding: 10px 14px;
  margin-bottom: 6px;
  transition: all 0.2s ease;
}

.account-card.has-token { border-color: #3d4f7c; }

.account-card.is-running {
  background: #1e2f1e;
  border-color: rgba(74, 222, 128, 0.4);
  box-shadow: 0 0 12px rgba(74, 222, 128, 0.12);
}

.account-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.account-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.account-status-dot.online  { background: #4ade80; }
.account-status-dot.offline { background: #4b5563; }
.account-status-dot.running {
  background: #4ade80;
  box-shadow: 0 0 6px #4ade80;
  animation: pulse 2s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.account-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.account-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 14px;
  font-weight: 600;
  color: #f1f5f9;
}

.running-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 6px;
  background: rgba(74, 222, 128, 0.15);
  color: #4ade80;
  font-weight: 500;
}

.game-badge {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 6px;
  background: rgba(100, 116, 139, 0.12);
  color: #94a3b8;
  font-weight: 500;
  white-space: nowrap;
}

.account-sub {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: #64748b;
}

.text-green { color: #86efac; }
.text-red   { color: #f87171; }

.running-time {
  color: #4ade80;
  font-family: 'SF Mono', 'Cascadia Code', 'Courier New', monospace;
  font-size: 10px;
  padding: 1px 5px;
  background: rgba(74, 222, 128, 0.08);
  border-radius: 3px;
}

.account-actions {
  display: flex;
  gap: 5px;
  margin-left: auto;
  flex-shrink: 0;
}

.btn {
  padding: 5px 12px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: all 0.12s;
}

.btn:active:not(:disabled) { transform: scale(0.96); }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-sm { padding: 4px 10px; font-size: 11px; }

.btn-primary { background: #3b82f6; color: #fff; }
.btn-primary:not(:disabled):hover { background: #2563eb; }

.btn-secondary { background: #1e2a3a; color: #94a3b8; border: 1px solid #2d3050; }
.btn-secondary:not(:disabled):hover { background: #253348; color: #e2e8f0; }

.btn-ghost { background: transparent; color: #64748b; }
.btn-ghost:hover { color: #e2e8f0; }

.btn-danger { background: transparent; color: #f87171; }
.btn-danger:hover { background: rgba(220, 38, 38, 0.12); }

.btn-danger-solid { background: #dc2626; color: #fff; }
.btn-danger-solid:hover { background: #b91c1c; }
</style>
