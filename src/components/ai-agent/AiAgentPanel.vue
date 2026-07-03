<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useAiStore } from '@/stores/ai'
import { useAgentChat } from './composables/useAgentChat'

const ai = useAiStore()
const win = getCurrentWindow()

const {
  messages, userInput, sending,
  enableDeepThink, enableLocalKb,
  selectedCombinedModel, modelOptions,
  activeChatRoleId, chatRolesList,
  handleModelChange, switchSession,
  sendMessage, clearChat, init, destroy,
} = useAgentChat()

const maximized = ref(false)
async function doMinimize() { try { await win.minimize() } catch {} }
async function doMaximize() {
  maximized.value = !maximized.value
  try { await win.toggleMaximize() } catch {}
}
async function doClose() { try { await win.close() } catch {} }

// ── Show settings modal ──
const showSettings = ref(false)

// ── Category tabs ──
const activeCategory = ref('daily')
const categories = [
  { id: 'daily', label: '日常办公' },
  { id: 'code', label: '代码开发' },
  { id: 'design', label: '设计创意' },
]

// ── Chat Mode: CRAFT / ASK / PLAN ──
const chatMode = ref('问答')
const chatModes = ['执行', '问答', '规划']

// ── Role selector (系统预设角色) ──
// activeChatRoleId / chatRolesList 来自 useAgentChat

// ── Quick action pills ──
const quickActions = [
  { id: 'skills', label: '技能', icon: '🧩' },
]

function selectSession(id: string) {
  switchSession(id)
}

function formatSessionDate(dateStr: string): string {
  if (!dateStr) return ''
  const d = new Date(dateStr)
  const now = new Date()
  const diffDays = Math.floor((now.getTime() - d.getTime()) / 86400000)
  if (diffDays === 0) return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  if (diffDays === 1) return '昨天'
  if (diffDays < 7) return `${diffDays}天前`
  return d.toLocaleDateString([], { month: 'numeric', day: 'numeric' })
}

const workspaceShortName = computed(() => {
  const p = ai.workspaceRoot
  if (!p) return '选择工作区'
  const parts = p.replace(/\\/g, '/').split('/')
  return parts[parts.length - 1] || p
})

async function handleSelectWorkspace() {
  await ai.changeWorkspaceDialog()
}

function handleKeydown(e: KeyboardEvent) {
  if (e.shiftKey) return
  e.preventDefault()
  sendMessage()
}

const hasMessages = computed(() => messages.value.length > 1 || (messages.value[0]?.role !== 'assistant'))

function formatMsgTime(): string {
  return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

let themeTimer: number | null = null

onMounted(() => {
  // 从 localStorage 同步主题
  const savedTheme = localStorage.getItem('jc9-theme')
  if (savedTheme === 'light' || savedTheme === 'dark') {
    document.documentElement.setAttribute('data-theme', savedTheme)
  }
  // 轮询监听主窗口主题变更
  themeTimer = window.setInterval(() => {
    const saved = localStorage.getItem('jc9-theme')
    if (saved) {
      const current = document.documentElement.getAttribute('data-theme')
      if (current !== saved) {
        document.documentElement.setAttribute('data-theme', saved)
      }
    }
  }, 2000)
  init()
})

onUnmounted(() => {
  if (themeTimer !== null) clearInterval(themeTimer)
  destroy()
})
</script>

<template>
  <div class="agent-window">
    <!-- ═══ Left Sidebar (W=250px) ═══ -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <svg class="sidebar-logo" viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="var(--jc-color-accent, #8a58ff)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
        </svg>
        <span class="logo-text">JC9</span>
        <span class="header-spacer"></span>
        <button class="header-icon-btn" @click="showSettings = !showSettings" title="设置">
          <svg viewBox="0 0 18 18" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="9" cy="9" r="2.5"></circle>
            <path d="M9 1.5v2M9 14.5v2M1.5 9h2M14.5 9h2M3.3 3.3l1.4 1.4M13.3 13.3l1.4 1.4M3.3 14.7l1.4-1.4M13.3 4.7l1.4-1.4"></path>
          </svg>
        </button>
      </div>

      <button class="new-task-btn" @click="clearChat">
        <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="10" y1="4" x2="10" y2="16"/><line x1="4" y1="10" x2="16" y2="10"/></svg>
        新建对话
      </button>

      <div class="session-list">
        <template v-for="s in ai.sessions" :key="s.id">
          <button :class="['session-item', { active: s.id === ai.currentSessionId }]" @click="selectSession(s.id)">
            <svg class="sess-icon" viewBox="0 0 18 18" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <path d="M15 12.5a1.5 1.5 0 0 1-1.5 1.5h-9l-3 3V4a1.5 1.5 0 0 1 1.5-1.5h10.5A1.5 1.5 0 0 1 15 4z"></path>
            </svg>
            <span class="sess-title">{{ s.title }}</span>
            <span class="sess-date">{{ formatSessionDate(s.updatedAt) }}</span>
          </button>
        </template>
        <div v-if="ai.sessions.length === 0" class="session-empty">暂无对话记录</div>
      </div>
    </aside>

    <!-- ═══ Main Content (flex:1) ═══ -->
    <div class="main-content">
      <!-- macOS-style TitleBar -->
      <div class="main-titlebar" data-tauri-drag-region>
        <div class="mtb-spacer"></div>
        <div class="mtb-controls">
          <button class="mtb-btn" @click="doMinimize" title="最小化">
            <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M3 13h10"/></svg>
          </button>
          <button class="mtb-btn" @click="doMaximize" title="最大化">
            <svg v-if="!maximized" viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="2.5" y="2.5" width="11" height="11" rx="1.5"/></svg>
            <svg v-else viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5"><rect x="3.5" y="5.5" width="7" height="7" rx="1"/><path d="M5.5 5.5V3.5h7v7h-2"/></svg>
          </button>
          <button class="mtb-btn mtb-close" @click="doClose" title="关闭">
            <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="M4 4l8 8M12 4l-8 8"/></svg>
          </button>
        </div>
      </div>

      <!-- Header -->
      <div class="content-header">
        <div class="header-left">
          <h1 class="header-title">JC9</h1>
          <p class="header-subtitle">AI Agent</p>
        </div>
        <button class="score-btn">
          来成长计划积分
          <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 5 14 10 9 15"/></svg>
        </button>
      </div>

      <!-- Category Tabs -->
      <div class="category-tabs">
        <button v-for="cat in categories" :key="cat.id" :class="['cat-tab', { active: activeCategory === cat.id }]" @click="activeCategory = cat.id">
          {{ cat.label }}
        </button>
      </div>

      <!-- Chat Messages -->
      <div class="chat-area" :class="{ empty: !hasMessages }">
        <!-- Empty State -->
        <div v-if="!hasMessages" class="empty-state">
          <div class="empty-icon">
            <svg viewBox="0 0 60 60" width="60" height="60" fill="none" stroke="#c7c7cc" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="8" y="26" width="44" height="24" rx="4"></rect>
              <circle cx="30" cy="12" r="6"></circle>
              <path d="M30 18v8"></path>
              <line x1="20" y1="38" x2="20" y2="38" stroke-width="3"></line>
              <line x1="40" y1="38" x2="40" y2="38" stroke-width="3"></line>
            </svg>
          </div>
          <p class="empty-title">开始一段新对话</p>
          <p class="empty-desc">向 JC9 提问或描述你的任务，我将协助你完成</p>
          <div class="empty-suggestions">
            <span class="suggestion-chip">💬 帮我总结今天的笔记</span>
            <span class="suggestion-chip">⚡ 优化这段代码</span>
            <span class="suggestion-chip">🧩 查找相关文档</span>
          </div>
        </div>

        <!-- Messages -->
        <div v-else class="chat-messages">
          <div v-for="(msg, i) in messages" :key="i" :class="['msg-group', msg.role]">
            <!-- User -->
            <div v-if="msg.role === 'user'" class="msg user-msg">
              <div class="msg-bubble user-bubble">
                <div class="msg-text">{{ msg.content }}</div>
              </div>
              <div class="msg-time">{{ formatMsgTime() }}</div>
            </div>
            <!-- System -->
            <div v-else-if="msg.role === 'system'" class="msg system-msg">
              <span class="system-text">{{ msg.content }}</span>
            </div>
            <!-- Assistant -->
            <div v-else class="msg assistant-msg">
              <div class="msg-sender-row">
                <span class="sender-avatar">
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="#007aff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
                  </svg>
                </span>
                <span class="sender-name">JC9</span>
                <span class="sender-model" v-if="msg.modelName">{{ msg.modelName }}</span>
                <span class="msg-time">{{ formatMsgTime() }}</span>
              </div>
              <div class="msg-bubble assistant-bubble">
                <div class="msg-text" v-html="msg.content.replace(/\n/g, '<br/>')"></div>
              </div>
            </div>
          </div>
          <!-- Typing -->
          <div v-if="sending" class="msg-group assistant">
            <div class="msg assistant-msg">
              <div class="msg-sender-row">
                <span class="sender-avatar">
                  <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="#007aff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
                  </svg>
                </span>
                <span class="sender-name">JC9</span>
              </div>
              <div class="msg-bubble assistant-bubble">
                <div class="typing-dots"><span class="dot"></span><span class="dot"></span><span class="dot"></span></div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ChatInputWrapper -->
      <div class="chat-input-wrapper">
        <div class="chip-group">
          <!-- Mode: CRAFT / ASK / PLAN -->
          <div class="mode-selector">
            <button v-for="mode in chatModes" :key="mode" :class="['mode-btn', { active: chatMode === mode }]"
              @click="chatMode = mode"
              :title="mode === '执行' ? '读写执行，调用全部工具' : mode === '问答' ? '只读问答，不执行操作' : '多级任务拆解，P0-P4 优先级规划'">
              {{ mode }}
            </button>
          </div>
          <!-- Role selector -->
          <div class="role-selector">
            <select v-model="activeChatRoleId" class="role-select" title="选择角色">
              <option value="auto">🤖 智能路由</option>
              <option v-for="r in chatRolesList" :key="r.id" :value="r.id">{{ r.icon }} {{ r.name }}</option>
            </select>
          </div>
          <!-- Expert Group placeholder -->
          <button class="chip invite-chip" title="邀请专家团（即将推出）">
            <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M10 4v12M4 10h12"/>
            </svg>
            专家团
          </button>
          <!-- DeepThink & Local KB -->
          <button :class="['chip toggle-chip', { active: enableDeepThink }]" @click="enableDeepThink = !enableDeepThink" title="深度思考：展示思维链过程">
            <svg viewBox="0 0 18 18" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <ellipse cx="9" cy="9" rx="2.5" ry="7" transform="rotate(45 9 9)"></ellipse>
              <ellipse cx="9" cy="9" rx="2.5" ry="7" transform="rotate(-45 9 9)"></ellipse>
              <circle cx="9" cy="9" r="1.2" fill="currentColor"></circle>
            </svg>
            深度思考
          </button>
          <button :class="['chip toggle-chip', { active: enableLocalKb }]" @click="enableLocalKb = !enableLocalKb" title="本地知识库：检索笔记作为参考">
            <svg viewBox="0 0 18 18" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="9" cy="9" r="6.5"></circle>
              <line x1="2.5" y1="9" x2="15.5" y2="9"></line>
              <path d="M9 2.5a11 11 0 0 1 3 6.5 11 11 0 0 1-3 6.5 11 11 0 0 1-3-6.5 11 11 0 0 1 3-6.5z"></path>
            </svg>
            知识库
          </button>
          <!-- Skills -->
          <button v-for="action in quickActions" :key="action.id" class="chip" @click="null">
            {{ action.icon }} {{ action.label }}
          </button>
          <span class="chip-spacer"></span>
          <!-- Model (with Auto mode) -->
          <div class="model-selector">
            <svg class="model-icon" viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <rect x="3" y="10" width="14" height="7" rx="1.5"></rect>
              <circle cx="10" cy="5" r="2"></circle>
              <path d="M10 7v3"></path>
            </svg>
            <select v-model="selectedCombinedModel" @change="handleModelChange" class="model-select" title="切换模型">
              <option value="">⚡ 自动</option>
              <optgroup v-for="(models, providerName) in modelOptions" :key="providerName" :label="providerName">
                <option v-for="m in models" :key="m.name" :value="m.id">{{ m.label }}</option>
              </optgroup>
            </select>
          </div>
        </div>

        <div class="input-bar">
          <div class="input-field-wrap">
            <textarea
              v-model="userInput"
              class="text-input-field"
              rows="3"
              placeholder="今天你做什么？@ 引用对话文件，/ 调用技能与指令"
              @keydown.enter.prevent="handleKeydown"
            ></textarea>
          </div>
          <button class="send-btn" :disabled="!userInput.trim() || sending" @click="sendMessage">
            <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="10" y1="16" x2="10" y2="4"/><polyline points="5 9 10 4 15 9"/>
            </svg>
          </button>
        </div>
        <!-- Workspace bar -->
        <div class="workspace-bar">
          <button class="workspace-btn" @click="handleSelectWorkspace" :title="ai.workspaceRoot || '选择工作区'">
            <svg viewBox="0 0 20 20" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M18 16a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h4l2 3h6a2 2 0 0 1 2 2z"></path>
            </svg>
            <span>{{ workspaceShortName }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
/* ═══ JC9 AI Agent — Themed Design ═══ */
.agent-window {
  display: flex; height: 100vh;
  font-family: 'Inter', system-ui, -apple-system, sans-serif;
  background: var(--jc-bg-app); color: var(--jc-text-primary); overflow: hidden;
}

/* ── Sidebar ── */
.sidebar {
  width: 250px; min-width: 250px; height: 100vh;
  background: var(--jc-bg-elevated); display: flex; flex-direction: column;
  border-right: 1px solid var(--jc-border-default);
}
.sidebar-header {
  display: flex; align-items: center; gap: 8px; padding: 16px 14px 12px;
}
.sidebar-logo { flex-shrink: 0; color: var(--jc-color-accent); }
.logo-text { font-size: 16px; font-weight: 700; color: var(--jc-text-highlight); letter-spacing: -0.2px; }
.header-spacer { flex: 1; }
.header-icon-btn {
  width: 28px; height: 28px; display: flex; align-items: center; justify-content: center;
  background: transparent; border: none; border-radius: 6px;
  color: var(--jc-text-secondary); cursor: pointer; transition: all 0.12s;
  &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
}
.new-task-btn {
  display: flex; align-items: center; justify-content: center; gap: 6px;
  width: calc(100% - 28px); margin: 0 14px 12px; height: 44px;
  background: var(--jc-color-accent); color: #fff; border: none; border-radius: 10px;
  font-size: 13px; font-weight: 600; cursor: pointer; font-family: inherit; transition: all 0.15s;
  &:hover { opacity: 0.9; transform: translateY(-1px); box-shadow: 0 2px 8px rgba(0,0,0,0.2); }
  &:active { transform: translateY(0); }
}
.session-list { flex: 1; overflow-y: auto; padding: 0 6px; display: flex; flex-direction: column; gap: 1px; }
.session-item {
  display: flex; align-items: center; gap: 6px; width: 100%; padding: 8px 10px;
  background: transparent; border: none; border-radius: 6px; cursor: pointer;
  font-family: inherit; text-align: left; transition: all 0.12s; flex-shrink: 0;
  &:hover { background: var(--jc-bg-hover); }
  &.active { background: var(--jc-bg-selected); }
  .sess-icon { color: var(--jc-text-secondary); flex-shrink: 0; }
  .sess-title {
    flex: 1; font-size: 12px; color: var(--jc-text-primary); overflow: hidden;
    text-overflow: ellipsis; white-space: nowrap; min-width: 0;
  }
  .sess-date { font-size: 10px; color: var(--jc-text-secondary); flex-shrink: 0; opacity: 0.7; }
}
.session-empty { text-align: center; padding: 24px 12px; font-size: 12px; color: var(--jc-text-secondary); }

/* ── Main Content ── */
.main-content { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--jc-bg-app); }

.main-titlebar {
  display: flex; align-items: center; justify-content: space-between; height: 32px;
  flex-shrink: 0; -webkit-app-region: drag; padding: 0 4px;
}
.mtb-spacer { flex: 1; }
.mtb-controls { display: flex; gap: 1px; height: 100%; align-items: center; -webkit-app-region: no-drag; }
.mtb-btn {
  width: 34px; height: 100%; display: flex; align-items: center; justify-content: center;
  background: none; border: none; color: var(--jc-text-secondary); cursor: pointer; border-radius: 0; transition: background 80ms;
  &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
}
.mtb-close:hover { background: #e81123 !important; color: #fff !important; }

.content-header {
  display: flex; align-items: flex-start; justify-content: space-between; padding: 20px 32px 0;
}
.header-left { display: flex; flex-direction: column; gap: 2px; }
.header-title {
  font-size: 28px; font-weight: 700; color: var(--jc-color-accent); margin: 0; line-height: 1.2; letter-spacing: -0.4px;
}
.header-subtitle { font-size: 18px; font-weight: 400; color: var(--jc-text-secondary); margin: 0; line-height: 1.3; }
.score-btn {
  display: flex; align-items: center; gap: 6px; height: 34px; padding: 0 14px;
  background: var(--jc-color-success); color: #fff; border: none; border-radius: 8px;
  font-size: 12px; font-weight: 600; cursor: pointer; font-family: inherit; white-space: nowrap;
  transition: all 0.15s; margin-top: 6px;
  &:hover { opacity: 0.9; box-shadow: 0 2px 8px rgba(0,0,0,0.15); }
}

.category-tabs { display: flex; gap: 4px; padding: 14px 32px 12px; flex-shrink: 0; }
.cat-tab {
  height: 32px; padding: 0 14px; border: none; background: transparent; color: var(--jc-text-secondary);
  font-size: 12px; font-weight: 500; cursor: pointer; border-radius: 6px; font-family: inherit; transition: all 0.15s;
  &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
  &.active { background: var(--jc-bg-selected); color: var(--jc-color-accent); font-weight: 600; }
}

/* ── Chat Messages ── */
.chat-area {
  flex: 1; overflow-y: auto; padding: 4px 32px 8px; min-height: 0;
  &.empty { display: flex; align-items: center; justify-content: center; }
  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-track { background: transparent; }
  &::-webkit-scrollbar-thumb { background: var(--jc-border-default); border-radius: 2px; }
}

.empty-state { text-align: center; max-width: 400px; animation: fadeUp 0.4s ease-out; }
.empty-icon { margin-bottom: 16px; opacity: 0.4; }
.empty-title { font-size: 18px; font-weight: 600; color: var(--jc-text-highlight); margin: 0 0 6px; }
.empty-desc { font-size: 13px; color: var(--jc-text-secondary); margin: 0 0 20px; line-height: 1.5; }
.empty-suggestions { display: flex; flex-wrap: wrap; gap: 8px; justify-content: center; }
.suggestion-chip {
  padding: 6px 14px; border-radius: 20px; background: var(--jc-bg-elevated); border: 1px solid var(--jc-border-default);
  font-size: 12px; color: var(--jc-text-secondary); cursor: pointer; transition: all 0.12s;
  &:hover { border-color: var(--jc-color-accent); color: var(--jc-color-accent); background: var(--jc-bg-hover); }
}

.chat-messages { display: flex; flex-direction: column; gap: 16px; max-width: 760px; margin: 0 auto; width: 100%; }
.msg-group { display: flex; flex-direction: column; }
.msg { max-width: 82%; animation: fadeUp 0.25s ease-out; }
.user-msg { align-self: flex-end; }
.assistant-msg { align-self: flex-start; }
.system-msg { align-self: center; max-width: 70%; }

.msg-sender-row {
  display: flex; align-items: center; gap: 6px; margin-bottom: 5px; padding: 0 4px;
}
.sender-avatar {
  width: 24px; height: 24px; display: flex; align-items: center; justify-content: center;
  background: var(--jc-bg-selected); border-radius: 6px; flex-shrink: 0;
  color: var(--jc-color-accent);
}
.sender-name { font-size: 12px; font-weight: 600; color: var(--jc-text-primary); }
.sender-model { font-size: 10px; color: var(--jc-text-secondary); background: var(--jc-bg-panel); padding: 1px 6px; border-radius: 4px; font-family: monospace; }

.msg-bubble { padding: 11px 16px; font-size: 14px; line-height: 1.6; word-break: break-word; }
.user-bubble {
  background: var(--jc-color-accent); color: #fff; border-radius: 16px 16px 4px 16px;
  box-shadow: 0 1px 4px rgba(0,0,0,0.15);
}
.assistant-bubble {
  background: var(--jc-bg-elevated); color: var(--jc-text-primary); border-radius: 16px 16px 16px 4px;
  border: 1px solid var(--jc-border-default); box-shadow: 0 1px 3px rgba(0,0,0,0.03);
}
.msg-text { white-space: pre-wrap; }
.msg-time { font-size: 10px; color: var(--jc-text-secondary); margin-top: 4px; padding: 0 4px; opacity: 0.7; }
.system-text { font-size: 11px; color: var(--jc-text-secondary); text-align: center; padding: 4px 12px; background: var(--jc-bg-panel); border-radius: 8px; }

.typing-dots { display: flex; gap: 4px; padding: 4px 2px; align-items: center; }
.dot {
  width: 7px; height: 7px; border-radius: 50%; background: var(--jc-text-secondary);
  animation: typingBounce 1.4s infinite ease-in-out both;
  &:nth-child(1) { animation-delay: -0.32s; }
  &:nth-child(2) { animation-delay: -0.16s; }
  &:nth-child(3) { animation-delay: 0s; }
}

@keyframes fadeUp { from { opacity: 0; transform: translateY(10px); } to { opacity: 1; transform: translateY(0); } }
@keyframes typingBounce { 0%,80%,100% { transform: scale(0.6); opacity: 0.4; } 40% { transform: scale(1); opacity: 1; } }

/* ── ChatInputWrapper ── */
.chat-input-wrapper {
  flex-shrink: 0; margin: 0 32px 14px; padding: 10px 14px;
  background: #fff; border-radius: 12px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.04), 0 1px 2px rgba(0,0,0,0.02);
  display: flex; flex-direction: column; gap: 8px;
}
.chip-group { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
.chip {
  height: 32px; padding: 0 12px; border: 1px solid var(--jc-border-default); border-radius: 20px;
  background: var(--jc-bg-panel); color: var(--jc-text-secondary); font-size: 11.5px; font-weight: 500;
  cursor: pointer; font-family: inherit; white-space: nowrap; transition: all 0.12s;
  display: flex; align-items: center; gap: 4px;
  &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); border-color: var(--jc-border-strong); }
  &.active { background: var(--jc-color-accent); color: #fff; border-color: var(--jc-color-accent); }
}
.toggle-chip { font-size: 11px; }
.toggle-chip.active { background: var(--jc-color-accent); color: #fff; }
.chip-spacer { flex: 1; min-width: 4px; }

/* Mode selector: CRAFT / ASK / PLAN */
.mode-selector {
  display: flex; gap: 2px; height: 32px;
  background: #f5f5f7; border: 1px solid #e8e8ed; border-radius: 20px; padding: 2px;
  flex-shrink: 0;
}
.mode-btn {
  padding: 0 12px; border: none; border-radius: 16px; background: transparent;
  color: #8e8e93; font-size: 11px; font-weight: 600; cursor: pointer;
  font-family: inherit; white-space: nowrap; transition: all 0.12s;
  &:hover { color: #555; }
  &.active { background: #fff; color: #007aff; box-shadow: 0 1px 3px rgba(0,0,0,0.08); }
}

/* Role selector */
.role-selector { flex-shrink: 0; }
.role-select {
  height: 32px; padding: 0 10px; border: 1px solid #e8e8ed; border-radius: 20px;
  background: #f5f5f7; color: #555; font-size: 11.5px; font-weight: 500;
  cursor: pointer; font-family: inherit; outline: none; max-width: 130px;
  transition: all 0.12s;
  &:hover { border-color: #d1d1d6; background: #e8e8ed; }
  option { color: #1d1d1f; background: #fff; }
}
.invite-chip {
  border-style: dashed !important; color: #8e8e93 !important; font-size: 11px !important;
  &:hover { border-color: #007aff !important; color: #007aff !important; }
}

.model-selector {
  display: flex; align-items: center; gap: 4px;
  height: 32px; padding: 0 8px 0 10px;
  border: 1px solid #e8e8ed; border-radius: 20px;
  background: #f5f5f7; color: #555;
  cursor: pointer; transition: all 0.12s; flex-shrink: 0;
  &:hover { border-color: #d1d1d6; background: #e8e8ed; }
  .model-icon { color: #8e8e93; flex-shrink: 0; }
}
.model-select {
  appearance: none; -webkit-appearance: none;
  border: none; background: transparent; color: #555;
  font-size: 11px; font-family: inherit; font-weight: 500;
  cursor: pointer; outline: none; padding: 0 4px 0 2px;
  max-width: 110px;
  option { color: #1d1d1f; background: #fff; }
}

/* Textarea */
.text-input-field {
  width: 100%; padding: 10px 14px; border: none; border-radius: 10px;
  background: #f5f5f7; color: #1d1d1f; font-size: 14px; line-height: 1.5;
  font-family: inherit; outline: none; box-sizing: border-box; resize: none;
  transition: all 0.15s;
  &:focus { background: #fff; box-shadow: inset 0 0 0 1.5px #007aff; }
  &::placeholder { color: #aeaeb2; }
}

/* Workspace bar */
.workspace-bar {
  display: flex; align-items: center; gap: 6px; padding-top: 2px;
}
.workspace-btn {
  display: flex; align-items: center; gap: 4px; padding: 3px 10px;
  border: 1px solid #e8e8ed; border-radius: 6px;
  background: transparent; color: #8e8e93; font-size: 11px;
  cursor: pointer; font-family: inherit; transition: all 0.12s;
  &:hover { border-color: #d1d1d6; color: #555; background: #f5f5f7; }
  span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 180px; }
}
.input-bar { display: flex; align-items: flex-start; gap: 8px; }
.input-field-wrap { flex: 1; }
.send-btn {
  margin-top: 6px; width: 28px; height: 28px; display: flex; align-items: center; justify-content: center;
  background: transparent; border: none; border-radius: 6px; color: #c7c7cc;
  cursor: pointer; flex-shrink: 0; transition: all 0.12s;
  &:hover:not(:disabled) { color: #007aff; background: rgba(0,122,255,0.06); }
  &:disabled { cursor: not-allowed; }
  &:not(:disabled) { color: #007aff; }
}
</style>
