<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useProjectStore } from '@/stores/project'
import ProjectSidebar from '@/components/ProjectSidebar.vue'
import MainPanel from '@/components/MainPanel.vue'
import TitleBar from '@/components/TitleBar.vue'

const store = useProjectStore()
const sidebarCollapsed = ref(false)
const isSplash = ref(false)

let defaultTerminalTimer: any = null

onMounted(async () => {
  const win = getCurrentWindow()

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
  } else {
    // 后台加载主窗口数据
    await store.loadProjects()
    await store.initListeners()

    // 显示并聚焦主窗口
    await win.show()
    await win.setFocus()
    defaultTerminalTimer = setTimeout(() => store.startDefaultTerminal(), 200)

    // 优雅地关闭 logo (splash) 窗口
    const splashWin = await WebviewWindow.getByLabel('splash')
    if (splashWin) {
      await splashWin.close()
    }
  }
})
onUnmounted(() => {
  store.destroyListeners()
  if (defaultTerminalTimer) {
    clearTimeout(defaultTerminalTimer)
  }
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

  <!-- Main window -->
  <div v-else class="app">
    <TitleBar />
    <div class="app-body">
      <div class="sidebar-wrap" :class="{fold:sidebarCollapsed}">
        <ProjectSidebar v-show="!sidebarCollapsed" />
      </div>
      <div class="splitter" @click="sidebarCollapsed=!sidebarCollapsed" :title="sidebarCollapsed?'展开侧栏':'折叠侧栏'">
        <span class="splitter-arrow">{{ sidebarCollapsed?'▶':'◀' }}</span>
      </div>
      <MainPanel />
    </div>
  </div>
</template>
<style scoped lang="scss">
.app { display:flex; flex-direction:column; height:100vh; background:var(--jc-bg-app); }
.app-body { display:flex; flex:1; overflow:hidden; }
.sidebar-wrap { width:210px; min-width:210px; transition:width .15s; overflow:hidden; &.fold { width:0; min-width:0; } }
.splitter { width:8px; min-width:8px; background:var(--jc-bg-elevated); cursor:pointer; display:flex; align-items:center; justify-content:center; border-left:1px solid var(--jc-border-default); border-right:1px solid var(--jc-border-default);
  &:hover { background:var(--jc-color-accent); }
  &-arrow { font-size:8px; color:var(--jc-text-secondary); }
  &:hover &-arrow { color:var(--jc-color-white); }
}
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
