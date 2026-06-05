<script setup lang="ts">
import { ref } from 'vue'

interface ConfirmOptions {
  title?: string
  message: string
  confirmText?: string
  cancelText?: string
  danger?: boolean
}

const visible = ref(false)
const options = ref<ConfirmOptions>({ message: '' })
let resolveFn: ((value: boolean) => void) | null = null

function confirm(opts: ConfirmOptions): Promise<boolean> {
  options.value = {
    title: opts.title ?? '确认',
    message: opts.message,
    confirmText: opts.confirmText ?? '确认',
    cancelText: opts.cancelText ?? '取消',
    danger: opts.danger ?? false,
  }
  visible.value = true
  return new Promise((resolve) => {
    resolveFn = resolve
  })
}

function onConfirm() {
  visible.value = false
  resolveFn?.(true)
  resolveFn = null
}

function onCancel() {
  visible.value = false
  resolveFn?.(false)
  resolveFn = null
}

defineExpose({ confirm })
</script>

<template>
  <Teleport to="body">
    <Transition name="confirm">
      <div v-if="visible" class="confirm-overlay" @click.self="onCancel">
        <div class="confirm-dialog" :class="{ danger: options.danger }">
          <div class="confirm-header">
            <h3>{{ options.title }}</h3>
            <button class="close-btn" @click="onCancel" title="关闭">✕</button>
          </div>
          <div class="confirm-body">
            <p>{{ options.message }}</p>
          </div>
          <div class="confirm-actions">
            <button class="btn btn-ghost" @click="onCancel">{{ options.cancelText }}</button>
            <button
              class="btn"
              :class="options.danger ? 'btn-danger-solid' : 'btn-primary'"
              @click="onConfirm"
            >
              {{ options.confirmText }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100000;
}

.confirm-dialog {
  background: #1a1f35;
  border: 1px solid #2d3050;
  border-radius: 12px;
  width: 400px;
  max-width: 90vw;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.confirm-dialog.danger {
  border-color: rgba(248, 113, 113, 0.4);
}

.confirm-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 18px 24px;
  border-bottom: 1px solid #2d3050;
}

.confirm-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: #e2e8f0;
}

.close-btn {
  background: transparent;
  border: none;
  color: #94a3b8;
  font-size: 20px;
  line-height: 1;
  cursor: pointer;
  padding: 4px;
  border-radius: 4px;
  transition: all 0.15s;
}

.close-btn:hover {
  background: rgba(148, 163, 184, 0.15);
  color: #e2e8f0;
}

.confirm-body {
  padding: 20px 24px;
}

.confirm-body p {
  margin: 0;
  font-size: 14px;
  color: #cbd5e1;
  line-height: 1.6;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 14px 24px;
  border-top: 1px solid #2d3050;
}

.btn {
  padding: 8px 20px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.15s;
}

.btn-primary { background: #3b82f6; color: #fff; }
.btn-primary:hover { background: #2563eb; }
.btn-ghost { background: transparent; color: #94a3b8; border: 1px solid #374151; }
.btn-ghost:hover { color: #e2e8f0; border-color: #6b7280; }
.btn-danger-solid { background: #dc2626; color: #fff; }
.btn-danger-solid:hover { background: #b91c1c; }

.confirm-enter-active,
.confirm-leave-active { transition: all 0.2s ease; }

.confirm-enter-from,
.confirm-leave-to {
  opacity: 0;
  transform: scale(0.95);
}
</style>
