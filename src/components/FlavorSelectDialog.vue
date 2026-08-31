<template>
  <Teleport to="body">
    <Transition name="flavor-modal">
      <div v-if="visible" class="flavor-overlay" @click.self="emit('cancel')">
        <div class="flavor-dialog">
          <div class="dialog-header">
            <h3>🐺 选择客户端分支</h3>
            <button class="close-btn" @click="emit('cancel')" title="关闭">✕</button>
          </div>

          <div class="dialog-body">
            <p class="dialog-hint">
              「{{ accountName }}」检测到多个已安装分支，请选择要启动的客户端
            </p>
            <button
              v-for="f in options"
              :key="f.value"
              type="button"
              class="flavor-option"
              :class="{ active: selected === f.value }"
              @click="selected = f.value"
            >
              <span class="flavor-check">{{ selected === f.value ? '●' : '○' }}</span>
              <span class="flavor-name">{{ f.label }}</span>
              <span class="flavor-folder">{{ f.folder }}</span>
            </button>
          </div>

          <div class="dialog-footer">
            <button type="button" class="btn btn-ghost" @click="emit('cancel')">取消</button>
            <button type="button" class="btn btn-primary" @click="emit('confirm', selected)">
              🚀 启动
            </button>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import { WOW_FLAVOR_OPTIONS } from '../types'

const props = defineProps<{
  visible: boolean
  accountName: string
  /** 已安装分支代码列表 */
  flavors: string[]
  /** 预选分支（账号记忆值） */
  defaultFlavor?: string
}>()
const emit = defineEmits<{
  confirm: [flavor: string]
  cancel: []
}>()

const selected = ref('')

watch(
  () => props.visible,
  (v) => {
    if (v) {
      selected.value = props.flavors.includes(props.defaultFlavor || '')
        ? props.defaultFlavor!
        : props.flavors[0]
    }
  },
)

const options = computed(() =>
  props.flavors.map(
    (value) =>
      WOW_FLAVOR_OPTIONS.find((f) => f.value === value) ?? { value, label: value, folder: '' },
  ),
)
</script>

<style scoped>
.flavor-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10002;
}

.flavor-dialog {
  background: #161b2e;
  border: 1px solid #2d3050;
  border-radius: 12px;
  width: 380px;
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
  transition: background 0.15s;
}

.close-btn:hover { background: rgba(148, 163, 184, 0.12); color: #e2e8f0; }

.dialog-body { padding: 16px 20px; }

.dialog-hint {
  margin: 0 0 12px;
  font-size: 12px;
  color: #94a3b8;
  line-height: 1.5;
}

.flavor-option {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  margin-bottom: 6px;
  background: #0d1117;
  border: 1px solid #2d3050;
  border-radius: 8px;
  cursor: pointer;
  color: #cbd5e1;
  transition: all 0.12s;
  text-align: left;
}

.flavor-option:hover { border-color: #3b82f6; }

.flavor-option.active {
  border-color: #3b82f6;
  background: rgba(59, 130, 246, 0.08);
}

.flavor-check {
  color: #3b82f6;
  font-size: 12px;
  flex-shrink: 0;
}

.flavor-name {
  font-size: 13px;
  font-weight: 600;
  color: #e2e8f0;
}

.flavor-folder {
  margin-left: auto;
  font-size: 11px;
  color: #64748b;
  font-family: 'SF Mono', 'Cascadia Code', 'Courier New', monospace;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 20px 16px;
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

.flavor-modal-enter-active,
.flavor-modal-leave-active { transition: all 0.2s ease; }

.flavor-modal-enter-from,
.flavor-modal-leave-to {
  opacity: 0;
  transform: scale(0.96) translateY(-8px);
}
</style>
