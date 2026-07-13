<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { WebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useProjectStore } from '@/stores/project'

const store = useProjectStore()
const theme = ref<'dark'|'light'>('dark')
const atop = ref(false)
const maximized = ref(false)
const win = getCurrentWindow()
defineEmits<{ quickNote: [] }>()

let unlistenResized: (() => void) | null = null

onMounted(async () => {
  const t = document.documentElement.getAttribute('data-theme')
  if (t === 'light' || t === 'dark') theme.value = t
  localStorage.setItem('jc9-theme', theme.value)

  try {
    maximized.value = await win.isMaximized()
    unlistenResized = await win.onResized(async () => {
      maximized.value = await win.isMaximized()
    })
  } catch {}
})

onUnmounted(() => {
  if (unlistenResized) {
    unlistenResized()
  }
})

function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
  document.documentElement.setAttribute('data-theme', theme.value)
  localStorage.setItem('jc9-theme', theme.value)
}

async function toggleAlwaysOnTop() {
  atop.value = !atop.value
  try { await win.setAlwaysOnTop(atop.value) } catch {}
}

async function doMinimize() {
  try { await win.minimize() } catch {}
}

async function doMaximize() {
  maximized.value = !maximized.value
  try { await win.toggleMaximize() } catch {}
}

async function doClose() {
  try { await win.close() } catch {}
}

async function openSettingsWindow() {
  const { openSettingsWindow: openSettings } = await import('@/utils/openSettings')
  await openSettings()
}

async function openAiAgent() {
  try {

    // 先尝试获取已有窗口
    const existing = await WebviewWindow.getByLabel('ai-agent')
    if (existing) {
      try {
        await existing.show()
        await existing.setFocus()
        return
      } catch {
        // 窗口已关闭，getByLabel 返回残引用，忽略直接新建
      }
    }
    // 创建新窗口（若 label 已存在 Tauri 会自动聚焦）
    const win = new WebviewWindow('ai-agent', {
      url: '/',
      title: 'JC9 AI Agent',
      width: 1300,
      height: 800,
      minWidth: 900,
      minHeight: 600,
      decorations: false,
    })
    win.once('tauri://created', () => {
      console.log('AI Agent window created')
    })
    win.once('tauri://error', (e) => {
      console.error('AI Agent window error:', e)
    })
  } catch (e) {
    console.error('Failed to open AI Agent window:', e)
  }
}
</script>

<template>
  <div class="titlebar">
    <div class="tb-left">
      <!-- jc9 SVG logo -->
      <svg class="tb-icon" viewBox="0 0 800 800" width="30" height="30" fill="currentColor">
        <g transform="translate(146.710431,668.255454) scale(0.064986,-0.064986)">
          <path d="M3805 7343 c-33 -14 -475 -264 -870 -493 -49 -29 -263 -152 -475 -273 -376 -215 -693 -398 -1140 -657 -143 -84 -209 -142 -242 -214 l-23 -51 0 -1535 0 -1535 28 -57 c15 -32 48 -74 71 -95 24 -21 189 -124 367 -228 296 -173 328 -190 371 -190 41 0 52 5 80 33 l33 32 5 1513 5 1512 33 67 c39 79 71 109 188 176 100 56 298 170 614 354 118 69 431 250 695 403 264 153 532 308 595 345 63 37 214 124 335 194 121 70 232 139 248 154 20 19 27 35 27 65 0 64 12 56 -500 350 -241 138 -262 147 -340 146 -36 0 -83 -7 -105 -16z"/>
          <path d="M5389 6441 c-40 -13 -34 -9 -1139 -649 -410 -237 -529 -306 -970 -562 -135 -78 -305 -176 -379 -219 -161 -92 -211 -134 -248 -209 l-28 -57 0 -625 0 -625 35 -64 c21 -39 50 -75 75 -93 22 -16 195 -117 385 -225 190 -108 408 -232 485 -276 168 -95 210 -114 274 -123 99 -13 106 -10 566 258 138 79 311 180 385 223 74 42 176 101 225 130 366 214 596 345 618 351 18 4 36 1 53 -11 48 -31 52 -58 47 -304 -5 -213 -7 -229 -30 -282 -13 -31 -41 -72 -61 -92 -20 -19 -161 -107 -312 -195 -151 -87 -417 -241 -590 -342 -609 -355 -737 -428 -775 -440 -58 -19 -154 -16 -206 7 -51 23 -571 314 -894 502 -167 96 -196 102 -251 47 l-35 -35 3 -418 3 -418 27 -57 c14 -31 44 -73 65 -93 21 -20 130 -89 243 -153 113 -65 329 -189 480 -277 363 -209 384 -219 479 -219 97 1 105 4 443 202 249 145 475 277 1548 902 195 113 429 249 518 301 194 111 241 150 279 227 l28 57 3 1527 2 1527 -21 56 c-34 91 -99 149 -284 256 -200 115 -288 165 -545 314 -327 189 -303 178 -395 182 -44 1 -92 -1 -106 -6z m156 -1106 c84 -22 153 -77 195 -156 30 -56 34 -73 35 -139 0 -89 -30 -162 -92 -222 -21 -20 -227 -146 -458 -280 -1220 -710 -1189 -692 -1255 -714 -136 -44 -295 27 -360 162 -32 65 -38 164 -15 234 33 100 79 135 505 381 212 123 491 284 620 359 636 369 629 365 687 379 56 13 73 13 138 -4z"/>
        </g>
      </svg>
      <span class="tb-title">jc9</span>
      <!-- Mode tabs: 主程序 / AI -->
      <div class="mode-tabs">
        <button :class="['mode-tab', { active: store.mainMode === 'main' }]" @click="store.mainMode = 'main'">主程序</button>
        <button :class="['mode-tab', { active: store.mainMode === 'ai' }]" @click="store.mainMode = 'ai'">AI</button>
      </div>
    </div>
    <div class="tb-controls">
      <button class="tb-btn" @click="$emit('quickNote')" title="快速笔记 (Ctrl+Shift+N)">
        <svg viewBox="0 0 18 18" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M4 2h10a1 1 0 011 1v12a1 1 0 01-1 1H4a1 1 0 01-1-1V3a1 1 0 011-1z"/>
          <path d="M6 6h6M6 9h4M6 12h2"/>
        </svg>
      </button>
      <button class="tb-btn" @click="openSettingsWindow" title="设置">
        <svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M469.333333 60.693333a85.333333 85.333333 0 0 1 85.333334 0l326.826666 188.714667a85.333333 85.333333 0 0 1 42.666667 73.898667v377.386666a85.333333 85.333333 0 0 1-42.666667 73.898667L554.666667 963.306667a85.333333 85.333333 0 0 1-85.333334 0L142.506667 774.592a85.333333 85.333333 0 0 1-42.666667-73.898667v-377.386666a85.333333 85.333333 0 0 1 42.666667-73.898667z m42.666667 73.898667L185.173333 323.306667v377.386666L512 889.408l326.826667-188.714667v-377.386666L512 134.592zM512 341.333333a170.666667 170.666667 0 1 1 0 341.333334 170.666667 170.666667 0 0 1 0-341.333334z m0 85.333334a85.333333 85.333333 0 1 0 0 170.666666 85.333333 85.333333 0 0 0 0-170.666666z"/></svg>
      </button>
      <button class="tb-btn ai-agent-btn" @click="openAiAgent" title="AI Agent">
        <svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M512.6144 54.3232a64 64 0 0 1 30.464 120.32l-0.0512 31.0784c104.7552 4.5568 200.704 25.6 255.0784 54.8864 73.6256 39.5776 110.8992 115.5072 122.5728 241.8688l4.1984-0.1536c35.328 0 64 28.672 64 64v64c0 35.328-28.672 64-64 64l-4.9664-0.2048c-12.1344 106.2912-46.592 184.4224-99.9936 216.3712-80.2304 43.8272-156.5696 61.952-311.1936 62.3104-154.5728-0.3072-230.7072-18.5856-312.8832-63.488-51.968-31.0784-86.016-109.1584-98.0992-215.2448l-4.864 0.256c-35.328 0-64-28.672-64-64v-64c0-35.328 28.672-64 64-64l4.096 0.1536c11.776-126.3616 48.9984-202.3424 122.6752-241.92 54.9888-29.4912 152.6272-50.7904 259.328-54.9888v-32.768a64 64 0 0 1 33.6384-118.4768zM380.928 474.8288c-21.248 0-38.4 14.336-38.4 32v128c0 17.664 17.152 32 38.4 32 21.1968 0 38.4-14.336 38.4-32v-128c0-17.664-17.2032-32-38.4-32z m256 0c-21.248 0-38.4 14.336-38.4 32v128c0 17.664 17.152 32 38.4 32 21.1968 0 38.4-14.336 38.4-32v-128c0-17.664-17.2032-32-38.4-32z"/></svg>
      </button>
      <button class="tb-btn" :class="{on:atop}" @click="toggleAlwaysOnTop" title="置顶">
        <svg viewBox="0 0 1024 1024" width="14" height="14" fill="currentColor"><path d="M681.003149 829.646642c-5.14681 8.823103-12.744482 14.705172-22.91556 17.401121-10.171077 2.695948-19.729439 1.470517-28.552542-3.55375L410.550526 716.90699l-168.496763 231.60646c-2.941034 3.676293-6.862414 6.249698-11.396509 7.230043-4.534095 1.225431-8.823103 0.612716-12.867025-1.715603l-0.490173-0.36763c-9.190733-5.391896-12.009224-12.989569-8.455474-23.160646l104.161636-269.96245L106.76618 541.547813c-8.823103-5.14681-14.705172-12.744482-17.401121-22.91556-2.695948-10.171077-1.470517-19.729439 3.55375-28.552542 24.140991-41.787197 59.188318-71.810257 105.264524-89.946636 46.076206-18.013836 85.902714-17.40112 119.602066 2.083233L468.758499 140.831874c-17.76875-10.29362-29.165258-25.488965-34.802241-45.953663-5.391896-20.219612-3.063578-39.458878 7.1075-57.227628 10.29362-17.76875 25.488965-29.165258 45.953663-34.80224 20.219612-5.391896 39.458878-3.186121 57.227628 7.1075l326.699906 188.593831c17.76875 10.29362 29.165258 25.488965 34.802241 45.953663 5.391896 20.342155 3.186121 39.458878-7.1075 57.227628s-25.488965 29.165258-45.953663 34.802241c-20.464698 5.636983-39.458878 3.186121-57.227628-7.1075L644.607848 590.810139c33.699353 19.484353 54.286594 53.673878 61.516637 102.446033 7.475129 49.01724-0.857802 94.480731-25.121336 136.39047zM441.676474 451.846263l131.978919-228.665425c2.695948-4.656638 3.55375-9.558362 2.205776-14.337543-1.347974-4.901724-4.411552-8.578017-9.06819-11.273965-4.656638-2.695948-9.558362-3.55375-14.337542-2.205776-4.901724 1.347974-8.578017 4.411552-11.273966 9.068189L409.202552 433.097169c-2.695948 4.656638-3.55375 9.558362-2.205776 14.337543 1.347974 4.901724 4.411552 8.578017 9.06819 11.273965 4.656638 2.695948 9.558362 3.55375 14.337542 2.205776 4.656638-1.347974 8.578017-4.289009 11.273966-9.06819z"/></svg>
      </button>
      <button class="tb-btn" @click="toggleTheme" :title="theme === 'dark' ? '浅色主题' : '深色主题'">
        <svg v-if="theme==='dark'" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
        </svg>
        <svg v-else viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="5"/>
          <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/>
        </svg>
      </button>
      <button class="tb-btn" @click="doMinimize" title="最小化">
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3 13h10"/></svg>
      </button>
      <button class="tb-btn" @click="doMaximize" title="最大化">
        <svg v-if="!maximized" viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/></svg>
        <svg v-else viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3.5" y="5.5" width="7" height="7" rx="1"/><path d="M5.5 5.5V3.5h7v7h-2"/></svg>
      </button>
      <button class="tb-btn tb-close" @click="doClose" title="关闭">
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>
      </button>
    </div>
  </div>
</template>

<style scoped lang="scss">
@use "@/styles/mixins.scss" as *;

.titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 32px;
  background: var(--jc-titlebar-bg);
  border-bottom: 1px solid var(--jc-border-default);
  flex-shrink: 0;
  user-select: none;
  -webkit-app-region: drag;
  padding: 0 4px 0 8px;
}
.tb-left {
  display: flex;
  align-items: center;
  gap: 6px;
  -webkit-app-region: drag;
}
.tb-icon {
  color: var(--jc-color-accent);
  flex-shrink: 0;
}
.tb-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-secondary);
}
.mode-tabs {
  display: flex; gap: 2px; margin-left: 12px; align-items: center; height: 22px;
  background: var(--jc-bg-panel); border: 1px solid var(--jc-border-default); border-radius: 6px; padding: 1px;
}
.mode-tab {
  padding: 0 10px; height: 18px; border: none; border-radius: 4px; background: transparent;
  color: var(--jc-text-secondary); font-size: 10.5px; font-weight: 500; cursor: pointer;
  font-family: inherit; white-space: nowrap; transition: all 0.12s;
  &:hover { color: var(--jc-text-primary); }
  &.active { background: var(--jc-bg-elevated); color: var(--jc-color-accent); font-weight: 600; box-shadow: 0 1px 2px rgba(0,0,0,0.06); }
}
.tb-controls {
  display: flex;
  gap: 1px;
  height: 100%;
  align-items: center;
  -webkit-app-region: no-drag;
}
.tb-btn {
  width: 36px;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: none;
  border: none;
  color: var(--jc-text-secondary);
  cursor: pointer;
  border-radius: 0;
  transition: background 80ms;
  &:hover { background: var(--jc-titlebar-btn-hover); color: var(--jc-text-primary); }
  &.on { color: var(--jc-color-accent); }
}
.tb-close:hover {
  background: var(--jc-titlebar-close-hover) !important;
  color: var(--jc-color-white) !important;
}
</style>
