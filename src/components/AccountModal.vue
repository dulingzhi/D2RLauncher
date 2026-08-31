<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import type { Account } from '../types'
import { GAME_OPTIONS, WOW_FLAVOR_OPTIONS } from '../types'

const props = defineProps<{
  visible: boolean
  initial?: Account | null
}>()
const emit = defineEmits<{
  close: []
  save: [data: Omit<Account, 'id' | 'encrypted_token' | 'token_set_at'>]
}>()

const label = ref('')
const game = ref<string>('osic')
const flavor = ref<string>('retail')
const customArgs = ref('')
const windowX = ref<number | null>(null)
const windowY = ref<number | null>(null)
const windowWidth = ref<number | null>(null)
const windowHeight = ref<number | null>(null)

const isWow = computed(() => game.value === 'wow')

watch(
  () => props.visible,
  (v) => {
    if (v && props.initial) {
      label.value = props.initial.label
      game.value = props.initial.game || 'osic'
      flavor.value = props.initial.flavor || 'retail'
      customArgs.value = props.initial.custom_args
      windowX.value = props.initial.window_x
      windowY.value = props.initial.window_y
      windowWidth.value = props.initial.window_width
      windowHeight.value = props.initial.window_height
    } else if (v) {
      label.value = ''
      game.value = 'osic'
      flavor.value = 'retail'
      customArgs.value = ''
      windowX.value = null
      windowY.value = null
      windowWidth.value = null
      windowHeight.value = null
    }
  }
)

function submit() {
  if (!label.value.trim()) return

  const toNullableNumber = (val: any): number | null => {
    if (val === '' || val === null || val === undefined || Number.isNaN(val)) return null
    return Number(val)
  }

  emit('save', {
    label: label.value.trim(),
    game: game.value,
    flavor: flavor.value,
    custom_args: customArgs.value.trim(),
    window_x: toNullableNumber(windowX.value),
    window_y: toNullableNumber(windowY.value),
    window_width: toNullableNumber(windowWidth.value),
    window_height: toNullableNumber(windowHeight.value),
  })
}
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div v-if="visible" class="modal-overlay" @click.self="emit('close')">
        <div class="modal">
          <div class="modal-header">
            <h3>{{ initial ? '编辑账号' : '添加账号' }}</h3>
            <button class="close-btn" @click="emit('close')" title="关闭">✕</button>
          </div>

          <form @submit.prevent="submit" class="modal-form">
            <div class="field">
              <label>显示名称</label>
              <input v-model="label" placeholder="如：小号1" required />
            </div>

            <div class="field">
              <label>游戏</label>
              <select v-model="game">
                <option v-for="g in GAME_OPTIONS" :key="g.uid" :value="g.uid">
                  {{ g.icon }} {{ g.label }}
                </option>
              </select>
            </div>

            <div class="field" v-if="isWow">
              <label>默认客户端分支（启动时可再选）</label>
              <select v-model="flavor">
                <option v-for="f in WOW_FLAVOR_OPTIONS" :key="f.value" :value="f.value">
                  {{ f.label }}（{{ f.folder }}）
                </option>
              </select>
            </div>

            <div class="field">
              <label>自定义启动参数</label>
              <input v-model="customArgs" placeholder="如：-mod MyMod" />
            </div>

            <div class="field-row">
              <div class="field">
                <label>窗口 X</label>
                <input v-model.number="windowX" type="number" placeholder="居中" />
              </div>
              <div class="field">
                <label>窗口 Y</label>
                <input v-model.number="windowY" type="number" placeholder="居中" />
              </div>
            </div>

            <div class="field-row">
              <div class="field">
                <label>宽度</label>
                <input v-model.number="windowWidth" type="number" placeholder="默认" />
              </div>
              <div class="field">
                <label>高度</label>
                <input v-model.number="windowHeight" type="number" placeholder="默认" />
              </div>
            </div>

            <div class="modal-actions">
              <button type="button" class="btn btn-ghost" @click="emit('close')">取消</button>
              <button type="submit" class="btn btn-primary">保存</button>
            </div>
          </form>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10000;
}

.modal {
  background: #161b2e;
  border: 1px solid #2d3050;
  border-radius: 12px;
  width: 420px;
  max-width: 92vw;
  max-height: 85vh;
  overflow-y: auto;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 14px 20px;
  border-bottom: 1px solid #2d3050;
}

.modal-header h3 {
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

.close-btn:hover {
  background: rgba(148, 163, 184, 0.12);
  color: #e2e8f0;
}

.modal-form {
  padding: 16px 20px 20px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 12px;
}

.field-row {
  display: flex;
  gap: 10px;
  margin-bottom: 12px;
}

.field-row .field {
  flex: 1;
  margin-bottom: 0;
}

label {
  font-size: 11px;
  color: #64748b;
  font-weight: 500;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

input,
select {
  background: #0d1117;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 7px 10px;
  color: #e2e8f0;
  font-size: 13px;
  outline: none;
  transition: border-color 0.15s;
}

input:focus,
select:focus { border-color: #3b82f6; }

select { cursor: pointer; }

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.btn {
  padding: 7px 18px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: all 0.15s;
}

.btn-primary { background: #3b82f6; color: #fff; }
.btn-primary:hover { background: #2563eb; }

.btn-ghost { background: transparent; color: #64748b; }
.btn-ghost:hover { color: #e2e8f0; }

.modal-enter-active,
.modal-leave-active { transition: all 0.2s ease; }

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-8px);
}
</style>
