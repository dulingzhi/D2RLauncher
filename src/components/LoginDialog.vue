<template>
  <Teleport to="body">
    <Transition name="login-dialog">
      <div v-if="visible" class="login-overlay" @click.self="onCancel">
        <div class="login-dialog">
          <div class="dialog-header">
            <h3>🔐 Battle.net 自动登录</h3>
            <button class="close-btn" @click="onCancel" title="关闭">✕</button>
          </div>

          <div class="dialog-body">
            <div class="steps">
              <div class="step">
                <span class="step-num">1</span>
                <span>在弹出窗口中输入账号密码</span>
              </div>
              <div class="step">
                <span class="step-num">2</span>
                <span>完成登录验证</span>
              </div>
              <div class="step">
                <span class="step-num">3</span>
                <span><strong>自动提取 Token</strong> ✨</span>
              </div>
            </div>

            <div class="auto-notice">
              <div class="spinner"></div>
              <p>等待登录完成，Token 将自动提取</p>
            </div>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { watch, onUnmounted } from 'vue'
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
    unlistenToken = await listen<{ account_id: string; token: string }>(
      'token_captured',
      (event) => {
        if (event.payload.account_id === props.accountId) {
          emit('tokenCaptured', event.payload.token)
          cleanup()
        }
      }
    )

    unlistenCancel = await listen<{ account_id: string }>(
      'login_cancelled',
      (event) => {
        if (event.payload.account_id === props.accountId) {
          emit('cancel')
          cleanup()
        }
      }
    )

    await invoke('open_login_with_navigation', { accountId: props.accountId })
  } catch (error) {
    console.error('初始化登录失败:', error)
    emit('cancel')
  }
}

const cleanup = () => {
  if (unlistenToken) { unlistenToken(); unlistenToken = null }
  if (unlistenCancel) { unlistenCancel(); unlistenCancel = null }
}

const onCancel = () => { cleanup(); emit('cancel') }

watch(() => props.visible, async (newVal) => {
  if (newVal) await initLogin()
  else cleanup()
})

onUnmounted(() => { cleanup() })
</script>

<style scoped>
.login-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10001;
}

.login-dialog {
  background: #161b2e;
  border: 1px solid #2d3050;
  border-radius: 12px;
  width: 400px;
  max-width: 90vw;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.dialog-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 20px;
  border-bottom: 1px solid #1e2540;
}

.dialog-header h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
  color: #e2e8f0;
}

.close-btn {
  background: transparent;
  border: none;
  color: #64748b;
  font-size: 18px;
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 4px;
  transition: all 0.15s;
}

.close-btn:hover { background: rgba(148, 163, 184, 0.12); color: #e2e8f0; }

.dialog-body { padding: 20px; }

.steps {
  background: #0d1117;
  border-radius: 8px;
  padding: 14px 16px;
  margin-bottom: 14px;
}

.step {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.step:last-child { margin-bottom: 0; }

.step-num {
  background: linear-gradient(135deg, #3b82f6, #2563eb);
  color: white;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  font-weight: 700;
  flex-shrink: 0;
}

.step {
  color: #cbd5e1;
  font-size: 13px;
  line-height: 1.4;
}

.step strong { color: #fbbf24; }

.auto-notice {
  background: rgba(59, 130, 246, 0.08);
  border: 1px solid rgba(59, 130, 246, 0.2);
  border-radius: 8px;
  padding: 12px 16px;
  display: flex;
  align-items: center;
  gap: 12px;
}

.spinner {
  border: 2px solid #2d3050;
  border-top: 2px solid #3b82f6;
  border-radius: 50%;
  width: 20px;
  height: 20px;
  animation: spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes spin { to { transform: rotate(360deg); } }

.auto-notice p {
  margin: 0;
  color: #94a3b8;
  font-size: 12px;
  font-weight: 500;
}

.login-dialog-enter-active,
.login-dialog-leave-active { transition: all 0.2s ease; }

.login-dialog-enter-from,
.login-dialog-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-6px);
}
</style>
