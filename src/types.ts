// 账号类型定义
export interface Account {
  id: string
  label: string
  encrypted_token: string | null
  token_set_at: number | null  // Unix timestamp (seconds)
  /** 所属游戏 uid：osic(D2R) / wow，见后端 games.rs */
  game: string
  /** WoW 分支：retail / classic / classic_era（仅 wow 使用） */
  flavor: string
  custom_args: string
  window_x: number | null
  window_y: number | null
  window_width: number | null
  window_height: number | null
}

export interface Settings {
  /** 兼容旧配置的 D2R 路径（读取时后端已归一到 game_paths.osic） */
  game_path: string
  /** uid → 安装根目录 */
  game_paths: Record<string, string>
  handle_path: string
  launch_delay_secs: number
  wait_for_login: boolean
  login_timeout_secs: number
  rename_window: boolean
}

/** 界面用的游戏定义（与后端 games.rs 的 GAMES 对应） */
export const GAME_OPTIONS = [
  { uid: 'osic', label: '暗黑破坏神2：狱火重生', short: 'D2R', icon: '🔥' },
  { uid: 'wow', label: '魔兽世界', short: 'WoW', icon: '🐺' },
] as const

/** WoW 分支定义 */
export const WOW_FLAVOR_OPTIONS = [
  { value: 'retail', label: '正式服', folder: '_retail_' },
  { value: 'classic', label: '怀旧服', folder: '_classic_' },
  { value: 'classic_era', label: '经典探索服', folder: '_classic_era_' },
  { value: 'titan', label: '泰坦重铸', folder: '_classic_titan_' },
  { value: 'anniversary', label: '周年庆', folder: '_anniversary_' },
] as const

export function gameInfo(uid: string) {
  return GAME_OPTIONS.find((g) => g.uid === uid) ?? GAME_OPTIONS[0]
}

/** 分支短名（卡片徽标等紧凑场景用） */
export function wowFlavorLabel(flavor: string) {
  return WOW_FLAVOR_OPTIONS.find((f) => f.value === flavor)?.label ?? flavor
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
