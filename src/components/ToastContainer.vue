<script setup lang="ts">
import { useToast } from '../composables/useToast'

const { toasts, remove } = useToast()
</script>

<template>
  <Teleport to="body">
    <div class="toast-container">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast"
          :class="`toast-${toast.type}`"
          @click="remove(toast.id)"
        >
          <span class="toast-icon">
            <template v-if="toast.type === 'success'">✅</template>
            <template v-else-if="toast.type === 'error'">❌</template>
            <template v-else-if="toast.type === 'warning'">⚠️</template>
            <template v-else>ℹ️</template>
          </span>
          <span class="toast-message">{{ toast.message }}</span>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-container {
  position: fixed;
  top: 60px;
  right: 16px;
  z-index: 99999;
  display: flex;
  flex-direction: column;
  gap: 8px;
  pointer-events: none;
  max-width: 380px;
}

.toast {
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  border: 1px solid;
  cursor: pointer;
  backdrop-filter: blur(12px);
  animation: toastSlideIn 0.25s ease-out;
}

.toast-icon {
  flex-shrink: 0;
  font-size: 14px;
}

.toast-message {
  flex: 1;
  line-height: 1.4;
}

.toast-info {
  background: rgba(30, 58, 110, 0.92);
  border-color: rgba(96, 165, 250, 0.4);
  color: #bfdbfe;
}

.toast-success {
  background: rgba(20, 60, 30, 0.92);
  border-color: rgba(74, 222, 128, 0.4);
  color: #bbf7d0;
}

.toast-warning {
  background: rgba(60, 45, 10, 0.92);
  border-color: rgba(251, 191, 36, 0.4);
  color: #fef3c7;
}

.toast-error {
  background: rgba(60, 15, 15, 0.92);
  border-color: rgba(248, 113, 113, 0.4);
  color: #fecaca;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(40px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(40px);
}

.toast-enter-active {
  transition: all 0.25s ease-out;
}

.toast-leave-active {
  transition: all 0.2s ease-in;
}

@keyframes toastSlideIn {
  from {
    opacity: 0;
    transform: translateX(40px);
  }
  to {
    opacity: 1;
    transform: translateX(0);
  }
}
</style>
