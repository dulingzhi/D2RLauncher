<script setup lang="ts">
import { ref, watch } from 'vue'
import type { Account } from '../types'

const props = defineProps<{
  visible: boolean
  initial?: Account | null
}>()
const emit = defineEmits<{
  close: []
  save: [data: Omit<Account, 'id' | 'encrypted_token' | 'token_set_at'>]
}>()

const label = ref('')
const email = ref('')
const customArgs = ref('')
const windowX = ref<number | null>(null)
const windowY = ref<number | null>(null)

watch(
  () => props.visible,
  (v) => {
    if (v && props.initial) {
      label.value = props.initial.label
      email.value = props.initial.email
      customArgs.value = props.initial.custom_args
      windowX.value = props.initial.window_x
      windowY.value = props.initial.window_y
    } else if (v) {
      label.value = ''
      email.value = ''
      customArgs.value = ''
      windowX.value = null
      windowY.value = null
    }
  }
)

function submit() {
  if (!label.value.trim() || !email.value.trim()) return
  emit('save', {
    label: label.value.trim(),
    email: email.value.trim(),
    custom_args: customArgs.value.trim(),
    window_x: windowX.value,
    window_y: windowY.value,
  })
}
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay" @click.self="emit('close')">
      <div class="modal">
        <h3>{{ initial ? '编辑账号' : '添加账号' }}</h3>
        <form @submit.prevent="submit">
          <div class="field">
            <label>显示名称 *</label>
            <input v-model="label" placeholder="如：小号1" required />
          </div>
          <div class="field">
            <label>Battle.net CN 邮箱 *</label>
            <input v-model="email" type="email" placeholder="如：123456@qq.com" required />
          </div>
          <div class="field">
            <label>自定义启动参数</label>
            <input v-model="customArgs" placeholder="如：-mod MyMod" />
          </div>
          <div class="field-row">
            <div class="field">
              <label>窗口 X 坐标</label>
              <input v-model.number="windowX" type="number" placeholder="可选" />
            </div>
            <div class="field">
              <label>窗口 Y 坐标</label>
              <input v-model.number="windowY" type="number" placeholder="可选" />
            </div>
          </div>
          <div class="modal-actions">
            <button type="button" class="btn btn-ghost" @click="emit('close')">取消</button>
            <button type="submit" class="btn btn-primary">保存</button>
          </div>
        </form>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0,0,0,0.65);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.modal {
  background: #1a1f35;
  border: 1px solid #2d3050;
  border-radius: 12px;
  padding: 28px 32px;
  width: 420px;
  max-width: 95vw;
}
.modal h3 {
  margin: 0 0 20px;
  font-size: 18px;
  color: #e2e8f0;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 16px;
}
.field-row {
  display: flex;
  gap: 12px;
}
.field-row .field { flex: 1; }
label {
  font-size: 13px;
  color: #94a3b8;
}
input {
  background: #0f1220;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 8px 12px;
  color: #e2e8f0;
  font-size: 14px;
  outline: none;
}
input:focus { border-color: #3b82f6; }
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 8px;
}
.btn {
  padding: 8px 20px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
}
.btn-primary   { background: #3b82f6; color: #fff; }
.btn-primary:hover { background: #2563eb; }
.btn-ghost     { background: transparent; color: #94a3b8; border: 1px solid #374151; }
.btn-ghost:hover { color: #e2e8f0; }
</style>
