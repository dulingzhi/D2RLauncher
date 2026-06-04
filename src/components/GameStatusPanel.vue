<template>
  <div class="game-status-panel">
    <div class="status-header">
      <h3>🎮 游戏状态</h3>
      <div class="status-badge" :class="{ running: isRunning }">
        {{ isRunning ? `运行中 (${status.total_running})` : '未运行' }}
      </div>
    </div>

    <div v-if="isRunning" class="instances-list">
      <div
        v-for="instance in status.running_instances"
        :key="instance.process_id"
        class="instance-card"
      >
        <div class="instance-header">
          <span class="instance-id">{{ instance.account_id }}</span>
          <span class="instance-region">{{ instance.region.toUpperCase() }}</span>
        </div>
        <div class="instance-name">{{ instance.account_name }}</div>
        <div class="instance-info">
          <span class="pid">PID: {{ instance.process_id }}</span>
        </div>
      </div>
    </div>

    <div v-else class="empty-state">
      <p>暂无游戏实例运行</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

interface GameInstance {
  account_id: string
  account_name: string
  region: string
  process_id: number
  window_title: string
  start_time: string
  total_seconds: number
}

interface GameStatus {
  running_instances: GameInstance[]
  total_running: number
}

const status = ref<GameStatus>({
  running_instances: [],
  total_running: 0,
})

const isRunning = computed(() => status.value.total_running > 0)

let unlisten: UnlistenFn | null = null

// 获取游戏状态
async function fetchGameStatus() {
  try {
    const result = await invoke<GameStatus>('get_game_status')
    status.value = result
  } catch (error) {
    console.error('获取游戏状态失败:', error)
  }
}

// 启动监控
async function startMonitoring() {
  try {
    await invoke('start_game_monitoring')
    console.log('✅ 游戏监控已启动')
  } catch (error) {
    console.error('启动监控失败:', error)
  }
}

// 停止监控
async function stopMonitoring() {
  try {
    await invoke('stop_game_monitoring')
    console.log('🛑 游戏监控已停止')
  } catch (error) {
    console.error('停止监控失败:', error)
  }
}

onMounted(async () => {
  // 监听游戏状态更新事件
  unlisten = await listen<GameStatus>('game_status_update', (event) => {
    status.value = event.payload
    console.log('🎮 游戏状态更新:', event.payload)
  })

  // 初始获取状态
  await fetchGameStatus()

  // 启动后台监控
  await startMonitoring()
})

onUnmounted(async () => {
  if (unlisten) {
    unlisten()
  }
  await stopMonitoring()
})
</script>

<style scoped>
.game-status-panel {
  background: rgba(26, 26, 26, 0.8);
  border-radius: 16px;
  padding: 20px;
  backdrop-filter: blur(10px);
  border: 1px solid rgba(255, 255, 255, 0.1);
}

.status-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.status-header h3 {
  margin: 0;
  font-size: 18px;
  color: #fff;
}

.status-badge {
  padding: 6px 12px;
  border-radius: 20px;
  font-size: 13px;
  font-weight: 500;
  background: rgba(100, 100, 100, 0.3);
  color: #999;
  transition: all 0.3s ease;
}

.status-badge.running {
  background: rgba(76, 175, 80, 0.2);
  color: #4caf50;
  border: 1px solid rgba(76, 175, 80, 0.4);
}

.instances-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.instance-card {
  background: rgba(40, 40, 40, 0.6);
  border-radius: 12px;
  padding: 16px;
  border: 1px solid rgba(255, 255, 255, 0.05);
  transition: all 0.3s ease;
}

.instance-card:hover {
  background: rgba(50, 50, 50, 0.6);
  border-color: rgba(255, 165, 0, 0.3);
}

.instance-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.instance-id {
  font-size: 16px;
  font-weight: 600;
  color: #ffa500;
}

.instance-region {
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 12px;
  font-weight: 500;
  background: rgba(100, 150, 255, 0.2);
  color: #6496ff;
}

.instance-name {
  font-size: 14px;
  color: #ccc;
  margin-bottom: 8px;
}

.instance-info {
  display: flex;
  gap: 16px;
  font-size: 12px;
  color: #888;
}

.pid {
  font-family: 'Courier New', monospace;
}

.empty-state {
  text-align: center;
  padding: 40px 20px;
  color: #666;
}

.empty-state p {
  margin: 0;
  font-size: 14px;
}
</style>
