<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { Settings } from '../types'

const settings = ref<Settings>({
  game_path: '',
  handle_path: '',
  launch_delay_secs: 5,
})
const saving = ref(false)
const saved = ref(false)

onMounted(async () => {
  try {
    settings.value = await invoke('get_settings')
  } catch (e) {
    console.error(e)
  }
})

async function save() {
  saving.value = true
  try {
    await invoke('save_settings', { settings: settings.value })
    saved.value = true
    setTimeout(() => (saved.value = false), 2000)
  } catch (e) {
    console.error(e)
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="settings-panel">
    <h2>⚙️ 设置</h2>

    <div class="section">
      <h4>游戏路径</h4>
      <div class="field">
        <label>D2R.exe 所在目录</label>
        <input
          v-model="settings.game_path"
          placeholder="如：C:\Program Files (x86)\Battle.net\Games\Diablo II Resurrected"
        />
        <p class="hint">程序会在此目录下查找 D2R.exe 并启动</p>
      </div>
    </div>

    <div class="section">
      <h4>Handle64.exe 路径</h4>
      <div class="field">
        <label>Handle64.exe 完整路径（用于多开句柄处理）</label>
        <input
          v-model="settings.handle_path"
          placeholder="如：C:\tools\Handle\handle64.exe"
        />
        <p class="hint">
          从
          <a href="https://download.sysinternals.com/files/Handle.zip" target="_blank">
            Sysinternals Handle
          </a>
          下载，放到 Handle/ 目录后填写路径。留空则不处理互斥锁（可能无法多开）。
        </p>
      </div>
    </div>

    <div class="section">
      <h4>批量启动设置</h4>
      <div class="field">
        <label>批量启动间隔（秒）</label>
        <input v-model.number="settings.launch_delay_secs" type="number" min="1" max="60" />
        <p class="hint">批量启动多个账号时，每个账号之间的等待时间</p>
      </div>
    </div>

    <button class="btn btn-primary" :disabled="saving" @click="save">
      {{ saving ? '保存中...' : saved ? '✅ 已保存' : '保存设置' }}
    </button>
  </div>
</template>

<style scoped>
.settings-panel {
  padding: 24px;
  max-width: 680px;
}
h2 {
  margin: 0 0 24px;
  font-size: 20px;
  color: #e2e8f0;
}
.section {
  margin-bottom: 24px;
}
h4 {
  margin: 0 0 12px;
  font-size: 14px;
  color: #94a3b8;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  border-bottom: 1px solid #1e2030;
  padding-bottom: 6px;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}
label {
  font-size: 13px;
  color: #cbd5e1;
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
.hint {
  font-size: 12px;
  color: #64748b;
  margin: 0;
}
a { color: #60a5fa; }
.btn {
  padding: 9px 24px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  font-weight: 500;
}
.btn-primary { background: #3b82f6; color: #fff; }
.btn-primary:hover { background: #2563eb; }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
