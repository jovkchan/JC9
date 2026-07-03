import { defineStore } from 'pinia'
import { ref } from 'vue'

/**
 * 全局系统设置 Store
 * 控制 SystemSettings 模态框的显示/隐藏，
 * 可供任何组件在任何地方调用。
 */
export const useSettingsStore = defineStore('settings', () => {
  const showSettings = ref(false)

  function open() { showSettings.value = true }
  function close() { showSettings.value = false }
  function toggle() { showSettings.value = !showSettings.value }

  return { showSettings, open, close, toggle }
})
