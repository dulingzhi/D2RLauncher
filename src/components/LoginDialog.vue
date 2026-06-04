<template>
  <div v-if="visible" class="login-dialog-overlay" @click.self="onCancel">
    <div class="login-dialog">
      <div class="dialog-header">
        <h3>🔐 Battle.net 自动登录</h3>
        <button class="close-btn" @click="onCancel">✕</button>
      </div>
      <div class="dialog-body">
        <div class="login-message">
          <div class="status-icon">🚀</div>
          <h4>登录窗口已打开</h4>
          <p class="hint">请在弹出的窗口中完成 Battle.net 登录</p>
          
          <div class="steps">
            <div class="step">
              <span class="step-number">1</span>
              <span class="step-text">在登录窗口中输入账号密码</span>
            </div>
            <div class="step">
              <span class="step-number">2</span>
              <span class="step-text">完成登录验证</span>
            </div>
            <div class="step">
              <span class="step-number">3</span>
              <span class="step-text"><strong>自动提取 Token 并返回</strong> ✨</span>
            </div>
          </div>

          <div class="auto-notice">
            <div class="spinner"></div>
            <p>等待登录完成，Token 将自动提取...</p>
          </div>

          <p class="hint-note">💡 无需手动复制粘贴，登录成功后会自动获取 Token</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

const props = defineProps<{
  visible: boolean
  accountId: string
}>()

const emit = defineEmits<{
  tokenCaptured: [token: string]
  cancel: []
}>()

let unlistenToken: (() => void) | null = null
let unlistenCancel: (() => void) | null = null

const initLogin = async () => {
  try {
    // 监听 token 捕获事件
    unlistenToken = await listen<{ account_id: string; token: string }>(
      'token_captured',
      (event) => {
        if (event.payload.account_id === props.accountId) {
          console.log('✅ 自动捕获 token:', event.payload.token)
          emit('tokenCaptured', event.payload.token)
          cleanup()
        }
      }
    )

    // 监听登录窗口关闭事件
    unlistenCancel = await listen<{ account_id: string }>(
      'login_cancelled',
      (event) => {
        if (event.payload.account_id === props.accountId) {
          console.log('🚪 用户关闭了登录窗口')
          emit('cancel')
          cleanup()
        }
      }
    )

    // 调用后端命令打开登录页面（自动捕获模式）
    await invoke('open_login_with_navigation', { accountId: props.accountId })
    
    console.log('🚀 Battle.net 登录窗口已打开（自动捕获模式）')
    
  } catch (error) {
    console.error('❌ 初始化登录失败:', error)
    emit('cancel')
  }
}

const cleanup = () => {
  if (unlistenToken) {
    unlistenToken()
    unlistenToken = null
  }
  if (unlistenCancel) {
    unlistenCancel()
    unlistenCancel = null
  }
}

const onCancel = () => {
  cleanup()
  emit('cancel')
}

watch(() => props.visible, async (newVal) => {
  if (newVal) {
    await initLogin()
  } else {
    cleanup()
  }
})

onUnmounted(() => {
  cleanup()
})
</script>

<style scoped>
.login-dialog-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.login-dialog {
  background: #1a1a1a;
  border-radius: 16px;
  width: 90%;
  max-width: 520px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 20px 24px;
  border-bottom: 1px solid #333;
}

.dialog-header h3 {
  margin: 0;
  color: #fff;
  font-size: 18px;
  font-weight: 600;
}

.close-btn {
  background: transparent;
  border: none;
  color: #888;
  font-size: 24px;
  cursor: pointer;
  padding: 0;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  transition: all 0.2s;
}

.close-btn:hover {
  background: #333;
  color: #fff;
}

.dialog-body {
  padding: 32px 24px;
}

.login-message {
  color: #e2e8f0;
}

.status-icon {
  font-size: 48px;
  text-align: center;
  margin-bottom: 16px;
  animation: bounce 2s ease-in-out infinite;
}

@keyframes bounce {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-10px); }
}

.login-message h4 {
  color: #fff;
  margin: 0 0 8px 0;
  font-size: 18px;
  font-weight: 500;
  text-align: center;
}

.hint {
  font-size: 14px;
  color: #94a3b8;
  margin: 8px 0 24px;
  text-align: center;
}

.steps {
  background: #0f172a;
  border-radius: 12px;
  padding: 20px;
  margin-bottom: 24px;
}

.step {
  display: flex;
  align-items: flex-start;
  margin-bottom: 12px;
}

.step:last-child {
  margin-bottom: 0;
}

.step-number {
  background: linear-gradient(135deg, #3b82f6 0%, #2563eb 100%);
  color: white;
  width: 24px;
  height: 24px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: bold;
  margin-right: 12px;
  flex-shrink: 0;
}

.step-text {
  color: #cbd5e1;
  font-size: 14px;
  line-height: 1.6;
  padding-top: 2px;
}

.step-text strong {
  color: #fbbf24;
}

.auto-notice {
  background: linear-gradient(135deg, rgba(59, 130, 246, 0.1) 0%, rgba(37, 99, 235, 0.1) 100%);
  border: 1px solid rgba(59, 130, 246, 0.3);
  border-radius: 12px;
  padding: 20px;
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 16px;
}

.spinner {
  border: 3px solid #334155;
  border-top: 3px solid #3b82f6;
  border-radius: 50%;
  width: 32px;
  height: 32px;
  animation: spin 1s linear infinite;
  flex-shrink: 0;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.auto-notice p {
  margin: 0;
  color: #cbd5e1;
  font-size: 14px;
  font-weight: 500;
}

.hint-note {
  text-align: center;
  font-size: 12px;
  color: #64748b;
  margin: 0;
}
</style>
