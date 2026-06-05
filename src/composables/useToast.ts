import { ref } from 'vue'

export type ToastType = 'info' | 'success' | 'warning' | 'error'

export interface Toast {
  id: number
  type: ToastType
  message: string
  duration: number
}

const toasts = ref<Toast[]>([])
let nextId = 0

const DEFAULT_DURATIONS: Record<ToastType, number> = {
  info: 3000,
  success: 2500,
  warning: 4000,
  error: 5000,
}

export function useToast() {
  function add(
    type: ToastType,
    message: string,
    duration?: number,
  ) {
    const id = ++nextId
    const toast: Toast = {
      id,
      type,
      message,
      duration: duration ?? DEFAULT_DURATIONS[type],
    }
    toasts.value.push(toast)

    if (toast.duration > 0) {
      setTimeout(() => remove(id), toast.duration)
    }
  }

  function remove(id: number) {
    const idx = toasts.value.findIndex((t) => t.id === id)
    if (idx !== -1) toasts.value.splice(idx, 1)
  }

  function clear() {
    toasts.value = []
  }

  function update(id: number, message: string, duration?: number) {
    const toast = toasts.value.find((t) => t.id === id)
    if (toast) {
      toast.message = message
      if (duration !== undefined) {
        toast.duration = duration
      }
    }
  }

  function info(message: string, duration?: number) {
    return add('info', message, duration)
  }
  function success(message: string, duration?: number) {
    add('success', message, duration)
  }
  function warning(message: string, duration?: number) {
    add('warning', message, duration)
  }
  function error(message: string, duration?: number) {
    add('error', message, duration)
  }

  return { toasts, add, update, remove, clear, info, success, warning, error }
}
