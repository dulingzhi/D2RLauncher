// 账号类型定义
export interface Account {
  id: string
  label: string
  encrypted_token: string | null
  token_set_at: number | null  // Unix timestamp (seconds)
  custom_args: string
  window_x: number | null
  window_y: number | null
  window_width: number | null
  window_height: number | null
}

export interface Settings {
  game_path: string
  handle_path: string
  launch_delay_secs: number
  wait_for_login: boolean
  login_timeout_secs: number
  rename_window: boolean
}

// 导入模式：与 Rust 端 backup::ImportMode 的 snake_case 序列化保持一致
export type ImportMode = 'overwrite' | 'skip_existing' | 'replace_all'

// 导入结果统计（与 Rust 端 backup::ImportStats 对应）
export interface ImportResult {
  imported: number
  updated: number
  skipped: number
}

export function tokenAge(token_set_at: number | null): string {
  if (!token_set_at) return '未设置'
  const now = Math.floor(Date.now() / 1000)
  const diff = now - token_set_at
  if (diff < 60) return `${diff}秒前获取`
  if (diff < 3600) return `${Math.floor(diff / 60)}分钟前获取`
  if (diff < 86400) return `${Math.floor(diff / 3600)}小时前获取`
  return `${Math.floor(diff / 86400)}天前获取`
}
