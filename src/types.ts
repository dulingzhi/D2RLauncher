// 账号类型定义
export interface Account {
  id: string
  label: string
  email: string
  encrypted_token: string | null
  token_set_at: number | null  // Unix timestamp (seconds)
  custom_args: string
  window_x: number | null
  window_y: number | null
}

export interface Settings {
  game_path: string
  handle_path: string
  launch_delay_secs: number
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
