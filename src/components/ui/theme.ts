// 主题切换 composable（对齐 Ant Design 暗黑模式：动态切换、可持久化、系统偏好）
// 参考: https://ant.design/docs/react/customize-theme-cn（暗色算法）
//
// 机制：以 <html data-theme="dark|light"> 为唯一数据源，组件通过 CSS 变量响应。
//   - 与 JC9 现有 TitleBar 的 data-theme 机制天然兼容（都操作同一个 DOM 属性）
//   - 换肤只需 document.documentElement.setAttribute('data-theme', ...)
import { computed, ref } from 'vue'

export type JcTheme = 'dark' | 'light'

const STORAGE_KEY = 'jc9-theme'

function readInitial(): JcTheme {
  const attr = typeof document !== 'undefined' ? document.documentElement.getAttribute('data-theme') : null
  if (attr === 'dark' || attr === 'light') return attr
  const saved = typeof localStorage !== 'undefined' ? localStorage.getItem(STORAGE_KEY) : null
  if (saved === 'dark' || saved === 'light') return saved
  return 'dark'
}

const current = ref<JcTheme>(readInitial())

/** 应用主题：写 DOM + 持久化 */
export function applyJcTheme(theme: JcTheme) {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-theme', theme)
  }
  if (typeof localStorage !== 'undefined') {
    localStorage.setItem(STORAGE_KEY, theme)
  }
  current.value = theme
}

/** 切换暗/亮 */
export function toggleJcTheme() {
  applyJcTheme(current.value === 'dark' ? 'light' : 'dark')
}

/** 在组件中使用：const { isDark, toggle } = useJcTheme() */
export function useJcTheme() {
  return {
    theme: current,
    isDark: computed(() => current.value === 'dark'),
    setTheme: applyJcTheme,
    toggle: toggleJcTheme,
  }
}
