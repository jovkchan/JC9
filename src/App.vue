<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { invoke } from '@tauri-apps/api/core'
import { useProjectStore } from '@/stores/project'
import { useStatusStore } from '@/stores/status'
import MainPanel from '@/components/MainPanel.vue'
import TitleBar from '@/components/TitleBar.vue'
import StatusBar from '@/components/StatusBar.vue'
import IconNav from '@/components/nav/IconNav.vue'
import SectionPanel from '@/components/sections/SectionPanel.vue'
import AiAgentPanel from '@/components/ai-agent/AiAgentPanel.vue'
import SettingsPanel from '@/components/settings/SettingsPanel.vue'
import VersionDiffWindow from '@/components/notes/VersionDiffWindow.vue'
import AiHelper from '@/components/tools/AiHelper.vue'
import QuickNote from '@/components/tools/QuickNote.vue'
import NotificationPanel from '@/components/NotificationPanel.vue'
import ToastMessage from '@/components/ToastMessage.vue'
import { registerToastHandler } from '@/utils/toast'

const store = useProjectStore()
const status = useStatusStore()
const isSplash = ref(false)
const isAiAgent = ref(false)
const windowLabel = ref('')
const showQuickNote = ref(false)
const toastRef = ref<InstanceType<typeof ToastMessage>>()

// 键盘快捷键：Ctrl+Shift+N 打开快速笔记
function onGlobalKeydown(e: KeyboardEvent) {
  // 屏蔽 F5 刷新
  if (e.key === 'F5') {
    e.preventDefault()
    return
  }
  // 屏蔽 Ctrl+R / Cmd+R 刷新
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'r') {
    e.preventDefault()
    return
  }
  if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key === 'N') {
    e.preventDefault()
    showQuickNote.value = !showQuickNote.value
  }
  if (e.key === 'Escape' && showQuickNote.value) {
    showQuickNote.value = false
  }
}

// 全局屏蔽浏览器默认右键菜单（自定义右键菜单不受影响）
function onGlobalContextMenu(e: MouseEvent) {
  e.preventDefault()
}

// Watch project count
watch(() => store.projects.length, (n) => status.setProjectCount(n), { immediate: true })

onMounted(async () => {
  // 注册浮动消息提示
  if (toastRef.value) {
    registerToastHandler((t) => toastRef.value!.addToast(t))
  }

  const win = getCurrentWindow()
  windowLabel.value = win.label

  if (win.label === 'splash') {
    isSplash.value = true
    
    // 强制将所有层级背景设为透明，消除由于全局全局 CSS 导致的黑色背景闪烁
    document.documentElement.style.setProperty('background', 'transparent', 'important')
    document.body.style.setProperty('background', 'transparent', 'important')
    const appEl = document.getElementById('app')
    if (appEl) {
      appEl.style.setProperty('background', 'transparent', 'important')
    }

    // 确保样式设置后才显示窗口，使窗口启动时就完全透明
    await win.show()
    await win.setFocus()
  } else if (win.label === 'ai-agent') {
    isAiAgent.value = true
    // 被动等待用户通过 TitleBar 按钮打开，不在启动时主动弹出
  } else if (win.label === 'settings') {
    // 被动等待用户通过按钮打开，不在启动时主动弹出
  } else {
    status.pushMessage('🚀 JC9 启动中...', 'info', false)

    // 后台加载主窗口数据
    status.pushMessage('正在加载项目...', 'info', false)
    await store.loadProjects()
    await store.initListeners()

    status.setProjectCount(store.projects.length)
    status.pushMessage(`✅ 项目加载完成 — ${store.projects.length} 个项目`, 'success', false)

    // 加载笔记
    status.pushMessage('正在加载笔记...', 'info', false)
    try {
      const ns = await import('@/stores/notes')
      await ns.useNotesStore().loadGroups()
      await ns.useNotesStore().loadAllNotes()
      status.pushMessage(`✅ 笔记加载完成 — ${ns.useNotesStore().notes.length} 条笔记`, 'success', false)
    } catch (e) {
      status.pushMessage(`笔记加载失败: ${e}`, 'error', false)
    }

    // 从 Rust 拉取启动诊断日志
    try {
      const logs: Array<{ step: string; message: string; level: string; count?: number; rows?: string[][] }> = await invoke('get_startup_logs')
      if (logs.length > 0) {
        status.pushMessage(`📋 Rust 端诊断: ${logs.length} 条日志`, 'info', false)
      }
      for (const log of logs) {
        status.pushMessage(log.message, (log.level as any) || 'info', false)
        // 如果是原始数据行，展开行详情
        if (log.rows && log.rows.length > 0) {
          status.pushMessage(`  └─ 行数据: ${JSON.stringify(log.rows)}`, 'info', false)
        }
      }
    } catch (e) {
      status.pushMessage(`拉取启动日志失败: ${e}`, 'warn', false)
    }

    // 显示并聚焦主窗口
    await win.show()
    await win.setFocus()

    // 注册全局键盘快捷键
    document.addEventListener('keydown', onGlobalKeydown)
    // 屏蔽浏览器默认右键菜单（自定义右键菜单不受影响，因事件先经目标元素再冒泡到 document）
    document.addEventListener('contextmenu', onGlobalContextMenu)

    // 优雅地关闭 logo (splash) 窗口
    const splashWin = await WebviewWindow.getByLabel('splash')
    if (splashWin) {
      await splashWin.close()
    }
  }
})
onUnmounted(() => {
  store.destroyListeners()
  document.removeEventListener('keydown', onGlobalKeydown)
  document.removeEventListener('contextmenu', onGlobalContextMenu)
})
</script>
<template>
  <!-- Splash: spinning icon on transparent background -->
  <div v-if="isSplash" class="splash">
    <svg class="splash-icon" viewBox="0 0 800 800" width="135" height="135" fill="currentColor">
      <g transform="translate(146.710431,668.255454) scale(0.064986,-0.064986)">
        <path d="M3805 7343 c-33 -14 -475 -264 -870 -493 -49 -29 -263 -152 -475 -273 -376 -215 -693 -398 -1140 -657 -143 -84 -209 -142 -242 -214 l-23 -51 0 -1535 0 -1535 28 -57 c15 -32 48 -74 71 -95 24 -21 189 -124 367 -228 296 -173 328 -190 371 -190 41 0 52 5 80 33 l33 32 5 1513 5 1512 33 67 c39 79 71 109 188 176 100 56 298 170 614 354 118 69 431 250 695 403 264 153 532 308 595 345 63 37 214 124 335 194 121 70 232 139 248 154 20 19 27 35 27 65 0 64 12 56 -500 350 -241 138 -262 147 -340 146 -36 0 -83 -7 -105 -16z"/>
        <path d="M5389 6441 c-40 -13 -34 -9 -1139 -649 -410 -237 -529 -306 -970 -562 -135 -78 -305 -176 -379 -219 -161 -92 -211 -134 -248 -209 l-28 -57 0 -625 0 -625 35 -64 c21 -39 50 -75 75 -93 22 -16 195 -117 385 -225 190 -108 408 -232 485 -276 168 -95 210 -114 274 -123 99 -13 106 -10 566 258 138 79 311 180 385 223 74 42 176 101 225 130 366 214 596 345 618 351 18 4 36 1 53 -11 48 -31 52 -58 47 -304 -5 -213 -7 -229 -30 -282 -13 -31 -41 -72 -61 -92 -20 -19 -161 -107 -312 -195 -151 -87 -417 -241 -590 -342 -609 -355 -737 -428 -775 -440 -58 -19 -154 -16 -206 7 -51 23 -571 314 -894 502 -167 96 -196 102 -251 47 l-35 -35 3 -418 3 -418 27 -57 c14 -31 44 -73 65 -93 21 -20 130 -89 243 -153 113 -65 329 -189 480 -277 363 -209 384 -219 479 -219 97 1 105 4 443 202 249 145 475 277 1548 902 195 113 429 249 518 301 194 111 241 150 279 227 l28 57 3 1527 2 1527 -21 56 c-34 91 -99 149 -284 256 -200 115 -288 165 -545 314 -327 189 -303 178 -395 182 -44 1 -92 -1 -106 -6z m156 -1106 c84 -22 153 -77 195 -156 30 -56 34 -73 35 -139 0 -89 -30 -162 -92 -222 -21 -20 -227 -146 -458 -280 -1220 -710 -1189 -692 -1255 -714 -136 -44 -295 27 -360 162 -32 65 -38 164 -15 234 33 100 79 135 505 381 212 123 491 284 620 359 636 369 629 365 687 379 56 13 73 13 138 -4z"/>
      </g>
    </svg>
    
    <!-- 简易斜体黑体文字标志 -->
    <div class="splash-text">JC CLI NINE</div>
  </div>

  <!-- AI Agent 独立窗口 -->
  <AiAgentPanel v-else-if="isAiAgent" />

  <!-- Settings 独立窗口 -->
  <SettingsPanel v-else-if="windowLabel === 'settings'" />

  <!-- Version Diff 独立窗口 -->
  <VersionDiffWindow v-else-if="windowLabel === 'version-diff'" />

  <!-- Main window -->
  <div v-else class="app">
    <TitleBar @quick-note="showQuickNote = true" />
    <!-- AI 模式：全屏显示 AiHelper，隐藏侧栏和主面板 -->
    <div v-if="store.mainMode === 'ai'" class="ai-mode-body">
      <AiHelper />
    </div>
    <!-- 主程序模式：正常显示 -->
    <template v-else>
      <div class="app-body">
        <IconNav />
        <SectionPanel />
        <MainPanel />
      </div>
      <StatusBar />
    </template>
  </div>

  <!-- 快速笔记浮动窗口 -->
  <QuickNote v-if="showQuickNote" @close="showQuickNote = false" />

  <!-- 通知中心面板 -->
  <NotificationPanel />
  <ToastMessage ref="toastRef" />
</template>
<style scoped lang="scss">
.app { display:flex; flex-direction:column; height:100vh; background:var(--jc-bg-app); }
.app-body { display:flex; flex:1; overflow:hidden; }
.ai-mode-body { flex: 1; display: flex; overflow: hidden; }
.splash {
  position: fixed;
  inset: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 15px;
  background: transparent;
}
.splash-icon {
  color: var(--jc-color-accent, #8a58ff);
  animation: flip 1.2s cubic-bezier(0.4, 0, 0.2, 1) infinite;
}
.splash-text {
  font-family: 'Segoe UI', system-ui, SimHei, sans-serif;
  font-size: 22px;
  font-weight: 900;
  font-style: italic;
  color: var(--jc-color-accent, #8a58ff);
  text-shadow: 0 2px 10px rgba(138, 88, 255, 0.5);
  letter-spacing: 3px;
}
@keyframes flip { from { transform:rotateY(0deg); } to { transform:rotateY(360deg); } }
</style>
