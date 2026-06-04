<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import AccountCard from './components/AccountCard.vue'
import AccountModal from './components/AccountModal.vue'
import SettingsPanel from './components/SettingsPanel.vue'
import type { Account } from './types'

type Tab = 'accounts' | 'settings'

const tab = ref<Tab>('accounts')
const accounts = ref<Account[]>([])
const showModal = ref(false)
const editingAccount = ref<Account | null>(null)
const batchLaunching = ref(false)
const batchMsg = ref('')

async function loadAccounts() {
  accounts.value = await invoke('get_accounts')
}

onMounted(loadAccounts)

function openAddModal() {
  editingAccount.value = null
  showModal.value = true
}

function openEditModal(account: Account) {
  editingAccount.value = account
  showModal.value = true
}

async function handleSave(data: Omit<Account, 'id' | 'encrypted_token' | 'token_set_at'>) {
  if (editingAccount.value) {
    await invoke('update_account', {
      account: { ...editingAccount.value, ...data },
    })
  } else {
    await invoke('add_account', {
      label: data.label,
      email: data.email,
      customArgs: data.custom_args,
      windowX: data.window_x,
      windowY: data.window_y,
    })
  }
  showModal.value = false
  await loadAccounts()
}

async function handleDelete(id: string) {
  if (!confirm('确认删除该账号？')) return
  await invoke('delete_account', { id })
  await loadAccounts()
}

async function launchAll() {
  const ids = accounts.value
    .filter((a) => a.encrypted_token)
    .map((a) => a.id)

  if (!ids.length) {
    batchMsg.value = '没有已配置 Token 的账号'
    setTimeout(() => (batchMsg.value = ''), 3000)
    return
  }

  batchLaunching.value = true
  batchMsg.value = `批量启动 ${ids.length} 个账号中...`
  try {
    const settings: any = await invoke('get_settings')
    await invoke('launch_all_accounts', {
      accountIds: ids,
      gamePath: settings.game_path,
      handlePath: settings.handle_path,
      delaySecs: settings.launch_delay_secs,
    })
    batchMsg.value = '✅ 批量启动完成'
  } catch (e) {
    batchMsg.value = `❌ ${e}`
  } finally {
    batchLaunching.value = false
    setTimeout(() => (batchMsg.value = ''), 4000)
  }
}

const readyCount = () => accounts.value.filter((a) => a.encrypted_token).length
</script>

<template>
  <div class="app">
    <!-- 标题栏 -->
    <header class="topbar">
      <div class="topbar-title">
        <img src="/tauri.svg" class="topbar-icon" alt="" />
        <span>D2R CN 多开启动器</span>
      </div>
      <nav class="tabs">
        <button
          class="tab-btn"
          :class="{ active: tab === 'accounts' }"
          @click="tab = 'accounts'"
        >账号管理</button>
        <button
          class="tab-btn"
          :class="{ active: tab === 'settings' }"
          @click="tab = 'settings'"
        >⚙️ 设置</button>
      </nav>
    </header>

    <!-- 账号列表页 -->
    <main v-if="tab === 'accounts'" class="main-content">
      <div class="list-header">
        <div class="list-meta">
          共 {{ accounts.length }} 个账号，
          <span class="ready">{{ readyCount() }} 个已配置 Token</span>
        </div>
        <div class="list-actions">
          <button class="btn btn-secondary" @click="openAddModal">+ 添加账号</button>
          <button
            class="btn btn-primary"
            :disabled="batchLaunching || readyCount() === 0"
            @click="launchAll"
          >
            {{ batchLaunching ? '启动中...' : '🚀 全部启动' }}
          </button>
        </div>
      </div>

      <div v-if="batchMsg" class="batch-msg">{{ batchMsg }}</div>

      <div v-if="accounts.length === 0" class="empty">
        <p>暂无账号，点击「添加账号」开始</p>
      </div>

      <AccountCard
        v-for="account in accounts"
        :key="account.id"
        :account="account"
        @refresh="loadAccounts"
        @edit="openEditModal"
        @delete="handleDelete"
      />
    </main>

    <!-- 设置页 -->
    <main v-else-if="tab === 'settings'" class="main-content">
      <SettingsPanel />
    </main>

    <!-- 添加/编辑弹窗 -->
    <AccountModal
      :visible="showModal"
      :initial="editingAccount"
      @close="showModal = false"
      @save="handleSave"
    />
  </div>
</template>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }

body {
  background: #0d1117;
  color: #e2e8f0;
  font-family: 'Segoe UI', system-ui, sans-serif;
  font-size: 14px;
  height: 100vh;
  overflow: hidden;
}

#app { height: 100vh; display: flex; flex-direction: column; }

.app { display: flex; flex-direction: column; height: 100vh; }

.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  height: 52px;
  background: #161b2e;
  border-bottom: 1px solid #1e2540;
  flex-shrink: 0;
}
.topbar-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 16px;
  font-weight: 700;
  color: #f1f5f9;
}
.topbar-icon {
  width: 22px;
  height: 22px;
  filter: drop-shadow(0 0 6px #3b82f6aa);
}
.tabs { display: flex; gap: 4px; }
.tab-btn {
  padding: 6px 16px;
  background: transparent;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  color: #94a3b8;
  font-size: 13px;
  font-weight: 500;
  transition: background 0.15s, color 0.15s;
}
.tab-btn:hover { background: #1e2a45; color: #e2e8f0; }
.tab-btn.active { background: #1e3a6e; color: #60a5fa; }

.main-content {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}

.list-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.list-meta { font-size: 13px; color: #64748b; }
.list-meta .ready { color: #86efac; }
.list-actions { display: flex; gap: 8px; }

.batch-msg {
  background: #1e2030;
  border: 1px solid #2d3050;
  border-radius: 6px;
  padding: 8px 14px;
  margin-bottom: 12px;
  font-size: 13px;
  color: #cbd5e1;
}

.empty {
  text-align: center;
  padding: 60px 0;
  color: #475569;
  font-size: 15px;
}

.btn {
  padding: 7px 16px;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
}
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-primary   { background: #3b82f6; color: #fff; }
.btn-primary:not(:disabled):hover { background: #2563eb; }
.btn-secondary { background: #1e2a3a; color: #94a3b8; border: 1px solid #2d3050; }
.btn-secondary:hover { background: #253348; color: #e2e8f0; }

/* 滚动条 */
::-webkit-scrollbar { width: 6px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: #2d3050; border-radius: 3px; }
</style>


<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

</style>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

</style>