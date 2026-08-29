<script setup lang="ts">
import { ref, nextTick } from 'vue'
import type { ImportMode } from '../types'

export interface BackupDialogResult {
  password: string
  mode?: ImportMode
}

type DialogMode = 'export' | 'import'

const visible = ref(false)
const mode = ref<DialogMode>('export')
const password = ref('')
const confirmPassword = ref('')
const importMode = ref<ImportMode>('overwrite')
const errorMsg = ref('')
const passwordInput = ref<HTMLInputElement | null>(null)

let resolveFn: ((value: BackupDialogResult | null) => void) | null = null

const importModes: { key: ImportMode; label: string; hint: string }[] = [
  { key: 'overwrite', label: '覆盖更新（推荐）', hint: '同 id 账号用备份覆盖，新账号追加' },
  { key: 'skip_existing', label: '只导入新账号', hint: '已存在的账号跳过，保留本地版本' },
  { key: 'replace_all', label: '全部替换', hint: '清空当前所有账号后导入备份' },
]

function open(dialogMode: DialogMode): Promise<BackupDialogResult | null> {
  mode.value = dialogMode
  password.value = ''
  confirmPassword.value = ''
  importMode.value = 'overwrite'
  errorMsg.value = ''
  visible.value = true
  nextTick(() => passwordInput.value?.focus())
  return new Promise((resolve) => {
    resolveFn = resolve
  })
}

function submit() {
  if (password.value.length < 6) {
    errorMsg.value = '密码至少需要 6 个字符'
    return
  }
  if (mode.value === 'export' && password.value !== confirmPassword.value) {
    errorMsg.value = '两次输入的密码不一致'
    return
  }
  visible.value = false
  resolveFn?.({
    password: password.value,
    mode: mode.value === 'import' ? importMode.value : undefined,
  })
  resolveFn = null
}

function cancel() {
  visible.value = false
  resolveFn?.(null)
  resolveFn = null
}

defineExpose({ open })
</script>

<template>
  <Teleport to="body">
    <Transition name="backup-dialog">
      <div v-if="visible" class="backup-overlay" @click.self="cancel">
        <div class="backup-dialog" @keydown.enter="submit" @keydown.esc="cancel">
          <div class="backup-header">
            <h3>{{ mode === 'export' ? '📤 导出账号数据' : '📥 导入账号数据' }}</h3>
            <button class="close-btn" @click="cancel" title="关闭">✕</button>
          </div>

          <div class="backup-body">
            <p v-if="mode === 'export'" class="desc">
              备份文件包含全部账号、认证 Token 和窗口设置，用密码加密后才能解开。
              <strong>密码丢失将无法恢复</strong>。
            </p>
            <p v-else class="desc">
              输入导出时设置的密码以解密备份文件，Token 会在本机重新加密保存。
            </p>

            <div class="field">
              <label>密码</label>
              <input
                ref="passwordInput"
                v-model="password"
                type="password"
                placeholder="至少 6 个字符"
                autocomplete="new-password"
              />
            </div>

            <div v-if="mode === 'export'" class="field">
              <label>确认密码</label>
              <input v-model="confirmPassword" type="password" placeholder="再输入一次" autocomplete="new-password" />
            </div>

            <div v-if="mode === 'import'" class="field">
              <label>导入方式</label>
              <div class="mode-list">
                <label
                  v-for="m in importModes"
                  :key="m.key"
                  class="mode-option"
                  :class="{ active: importMode === m.key, danger: m.key === 'replace_all' }"
                >
                  <input v-model="importMode" type="radio" :value="m.key" name="import-mode" />
                  <div class="mode-text">
                    <span class="mode-label">{{ m.label }}</span>
                    <span class="mode-hint">{{ m.hint }}</span>
                  </div>
                </label>
              </div>
              <p v-if="importMode === 'replace_all'" class="danger-warn">
                ⚠️ 此操作会先删除当前所有账号（含 Token），且无法撤销
              </p>
            </div>

            <p v-if="errorMsg" class="error-msg">⚠️ {{ errorMsg }}</p>
          </div>

          <div class="backup-actions">
            <button class="btn btn-ghost" @click="cancel">取消</button>
            <button
              class="btn"
              :class="importMode === 'replace_all' && mode === 'import' ? 'btn-danger-solid' : 'btn-primary'"
              @click="submit"
            >
              {{ mode === 'export' ? '加密导出' : '解密导入' }}
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.backup-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100000;
}

.backup-dialog {
  background: #1a1f35;
  border: 1px solid #2d3050;
  border-radius: 12px;
  width: 440px;
  max-width: 90vw;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
  overflow: hidden;
}

.backup-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 18px 24px;
  border-bottom: 1px solid #2d3050;
}

.backup-header h3 {
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

.backup-body {
  padding: 20px 24px;
}

.desc {
  margin: 0 0 14px;
  font-size: 12px;
  color: #94a3b8;
  line-height: 1.6;
}

.desc strong {
  color: #f87171;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 14px;
}

.field > label {
  font-size: 13px;
  color: #cbd5e1;
}

input[type='password'] {
  background: #0d1117;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 8px 12px;
  color: #e2e8f0;
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
}

input[type='password']:focus {
  border-color: #3b82f6;
}

.mode-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.mode-option {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  background: #0d1117;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 9px 12px;
  cursor: pointer;
  transition: border-color 0.15s;
}

.mode-option:hover {
  border-color: #3b82f6;
}

.mode-option.active {
  border-color: #3b82f6;
  background: rgba(59, 130, 246, 0.08);
}

.mode-option.danger.active {
  border-color: #f87171;
  background: rgba(248, 113, 113, 0.08);
}

.mode-option input[type='radio'] {
  margin-top: 2px;
  accent-color: #3b82f6;
  cursor: pointer;
}

.mode-option.danger input[type='radio'] {
  accent-color: #f87171;
}

.mode-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.mode-label {
  font-size: 13px;
  color: #e2e8f0;
}

.mode-hint {
  font-size: 11px;
  color: #64748b;
}

.danger-warn {
  margin: 0;
  font-size: 11px;
  color: #f87171;
  line-height: 1.5;
}

.error-msg {
  margin: 0;
  font-size: 12px;
  color: #f87171;
}

.backup-actions {
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

.btn-primary {
  background: #3b82f6;
  color: #fff;
}
.btn-primary:hover {
  background: #2563eb;
}

.btn-ghost {
  background: transparent;
  color: #94a3b8;
  border: 1px solid #374151;
}
.btn-ghost:hover {
  color: #e2e8f0;
  border-color: #6b7280;
}

.btn-danger-solid {
  background: #dc2626;
  color: #fff;
}
.btn-danger-solid:hover {
  background: #b91c1c;
}

.backup-dialog-enter-active,
.backup-dialog-leave-active {
  transition: all 0.2s ease;
}

.backup-dialog-enter-from,
.backup-dialog-leave-to {
  opacity: 0;
  transform: scale(0.95);
}
</style>
