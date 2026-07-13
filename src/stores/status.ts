import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { toast } from '@/utils/toast'

export type ConnectionStatus = 'local' | 'online' | 'offline' | 'syncing'

export interface StatusMessage {
  id: string
  text: string
  type: 'info' | 'success' | 'warn' | 'error'
  timestamp: number
}

export const useStatusStore = defineStore('status', () => {
  // ── Connection ──
  const connectionStatus = ref<ConnectionStatus>('local')
  const connectionLabel = computed(() => {
    const map: Record<ConnectionStatus, string> = {
      local: '本地模式',
      online: '已连接',
      offline: '离线',
      syncing: '同步中',
    }
    return map[connectionStatus.value]
  })

  // ── Messages ──
  const messages = ref<StatusMessage[]>([])
  const currentMessage = computed(() => {
    if (messages.value.length === 0) return null
    return messages.value[messages.value.length - 1]
  })

  let msgId = 0
  function pushMessage(text: string, type: StatusMessage['type'] = 'info', showToast = true) {
    const msg: StatusMessage = {
      id: `${++msgId}`,
      text,
      type,
      timestamp: Date.now(),
    }
    messages.value.push(msg)
    // Keep max 50 messages in history
    if (messages.value.length > 50) {
      messages.value = messages.value.slice(-50)
    }
    if (showToast) toast({ message: text, type, duration: 3000 })
  }

  // ── Notifications ──
  const notificationCount = ref(0)
  const notificationPanelOpen = ref(false)
  function setNotificationCount(n: number) { notificationCount.value = n }
  function toggleNotificationPanel() { notificationPanelOpen.value = !notificationPanelOpen.value }
  function openNotificationPanel() { notificationPanelOpen.value = true }
  function closeNotificationPanel() { notificationPanelOpen.value = false }

  // ── User ──
  const userName = ref('')
  const isLoggedIn = computed(() => !!userName.value)
  function setUser(name: string) { userName.value = name }
  function clearUser() { userName.value = '' }

  // ── Stats ──
  const noteCount = ref(0)
  const projectCount = ref(0)
  function setNoteCount(n: number) { noteCount.value = n }
  function setProjectCount(n: number) { projectCount.value = n }

  return {
    connectionStatus,
    connectionLabel,
    messages,
    currentMessage,
    pushMessage,
    notificationCount,
    notificationPanelOpen,
    toggleNotificationPanel,
    openNotificationPanel,
    closeNotificationPanel,
    setNotificationCount,
    userName,
    isLoggedIn,
    setUser,
    clearUser,
    noteCount,
    projectCount,
    setNoteCount,
    setProjectCount,
  }
})
