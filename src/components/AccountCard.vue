<script setup lang="ts">
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Account } from '../types'
import { tokenAge } from '../types'
import LoginDialog from './LoginDialog.vue'

const props = defineProps<{ account: Account }>()
const emit = defineEmits<{
  refresh: []
  edit: [account: Account]
  delete: [id: string]
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
    statusMsg.value = '✅ Token 获取成功'
    emit('refresh')
  } catch (e) {
    statusMsg.value = `❌ 保存 Token 失败: ${e}`
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
  launching.value = true
  statusMsg.value = '正在启动...'
  try {
    const settings: any = await invoke('get_settings')
    await invoke('launch_account', {
      accountId: props.account.id,
      gamePath: settings.game_path,
      handlePath: settings.handle_path,
    })
    statusMsg.value = '✅ 已启动'
  } catch (e) {
    statusMsg.value = `❌ 启动失败: ${e}`
  } finally {
    launching.value = false
    setTimeout(() => (statusMsg.value = ''), 4000)
  }
}

const hasToken = computed(() => !!props.account.encrypted_token)
const tokenInfo = computed(() => tokenAge(props.account.token_set_at))
</script>

<template>
  <div class="account-card" :class="{ 'has-token': hasToken }">
    <div class="account-header">
      <div class="account-status-dot" :class="hasToken ? 'online' : 'offline'" />
      <div class="account-info">
        <span class="account-label">{{ account.label }}</span>
        <span class="account-email">{{ account.email }}</span>
      </div>
      <div class="account-token-info">
        <span :class="hasToken ? 'token-ok' : 'token-missing'">
          {{ hasToken ? `Token: ${tokenInfo}` : 'Token: 未设置' }}
        </span>
      </div>
    </div>

    <div v-if="statusMsg" class="status-msg">{{ statusMsg }}</div>

    <div class="account-actions">
      <button
        class="btn btn-primary"
        :disabled="!hasToken || launching"
        @click="launch"
      >
        {{ launching ? '启动中...' : '🚀 启动' }}
      </button>
      <button
        class="btn btn-secondary"
        :disabled="gettingToken"
        @click="getToken"
      >
        {{ gettingToken ? '等待登录...' : hasToken ? '🔄 刷新 Token' : '🔑 获取 Token' }}
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
  transition: border-color 0.2s;
}
.account-card.has-token {
  border-color: #3d4f7c;
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
.account-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
}
.account-label {
  font-weight: 600;
  font-size: 15px;
  color: #e2e8f0;
}
.account-email {
  font-size: 12px;
  color: #94a3b8;
}
.account-token-info {
  font-size: 12px;
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
</style>
