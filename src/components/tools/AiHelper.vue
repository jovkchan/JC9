<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useAiStore } from '@/stores/ai'
import { useAiHelper } from './composables/useAiHelper'
import { deleteChatMessages } from '@/utils/chatStorage'
import JcModal from '@/components/ui/JcModal.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcSearchIcon from '@/components/ui/JcSearchIcon.vue'

const notesStore = useNotesStore()
const ai = useAiStore()

const {
  messages, userInput, sending, inputTextarea,
  chatMode, chatModes,
  enableDeepThink, enableLocalKb,
  aiProvider, aiModel,
  selectedCombinedModel, modelOptions, loadingModels,
  activeChatRoleId, chatRolesList,
  isFocused,
  showBrowserDialog, browserUrlInput,
  isConsoleExpanded, expandedWorkers,
  kbSearchQuery, kbSearchResults,
  placeholderText, workspaceShortName,
  handleModelChange,
  refreshLocalModels,
  handleEnterKey, sendMessage, clearChat,
  attachActiveNote, polishMemo, recommendTags,
  handleApprove, handleDeny, handleApproveAll, handleDenyAll,
  handleKillWorker, handleKillAllWorkers,
  toggleWorkerExpand, getTaskTitle, getWorkerRole,
  formatTime, statusColors, riskColors, statusLabel,
  searchKnowledgeBase,
  handleBrowserOpen, handleBrowserConfirm,
  handleSelectWorkspace, selectSession,
  autoResizeTextarea,
  init, destroy,
} = useAiHelper()

// ── Custom confirm dialog ──
const confirmDelete = ref<{ show: boolean; sessionId: string; title: string }>({ show: false, sessionId: '', title: '' })

function requestDeleteSession(sessionId: string, title: string) {
  confirmDelete.value = { show: true, sessionId, title }
}

async function executeDeleteSession() {
  const { sessionId } = confirmDelete.value
  confirmDelete.value = { show: false, sessionId: '', title: '' }
  await ai.deleteSession(sessionId)
  await deleteChatMessages(sessionId)
}

// ── Computed for template ──
const hasContent = computed(() => userInput.value.trim().length > 0)

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

async function openStandaloneSettings() {
  const { openSettingsWindow } = await import('@/utils/openSettings')
  await openSettingsWindow()
}



onMounted(() => { init() })
onUnmounted(() => { destroy() })
</script>

<template>
  <div class="ai-helper-container">
    <!-- ═══ Left Sidebar (W=250px) ═══ -->
    <aside class="sidebar">
      <div class="sidebar-header">
      


      </div>

      <button class="new-chat-btn" @click="clearChat">
        <svg viewBox="0 0 20 20" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="10" y1="4" x2="10" y2="16"/><line x1="4" y1="10" x2="16" y2="10"/></svg>
        新建对话
      </button>

      <div class="session-list">
        <template v-for="s in ai.sessions" :key="s.id">
          <div :class="['session-item-wrap', { active: s.id === ai.currentSessionId }]">
            <button class="session-item" @click="selectSession(s.id)">
              <svg class="sess-icon" viewBox="0 0 18 18" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                <path d="M15 12.5a1.5 1.5 0 0 1-1.5 1.5h-9l-3 3V4a1.5 1.5 0 0 1 1.5-1.5h10.5A1.5 1.5 0 0 1 15 4z"></path>
              </svg>
              <span class="sess-title">{{ s.title }}</span>
              <span class="sess-date">{{ formatSessionDate(s.updatedAt) }}</span>
            </button>
            <button class="session-del-btn" @click.stop="requestDeleteSession(s.id, s.title)" title="删除对话">✕</button>
          </div>
        </template>
        <div v-if="ai.sessions.length === 0" class="session-empty">暂无对话记录</div>
      </div>

      <!-- Sidebar Footer -->
      <div class="sidebar-footer">
        <button class="sidebar-footer-btn" @click="handleBrowserOpen" title="打开浏览器">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:14px;height:14px">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="2" y1="12" x2="22" y2="12"></line>
            <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
          </svg>
          <span>浏览器</span>
        </button>
        <button class="sidebar-footer-btn" @click="openStandaloneSettings" title="设置">
          <svg viewBox="0 0 18 18" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="9" cy="9" r="2.5"></circle>
            <path d="M9 1.5v2M9 14.5v2M1.5 9h2M14.5 9h2M3.3 3.3l1.4 1.4M13.3 13.3l1.4 1.4M3.3 14.7l1.4-1.4M13.3 4.7l1.4-1.4"></path>
          </svg>
          <span>设置</span>
        </button>
      </div>
    </aside>

    <!-- ═══ Main Chat Area ═══ -->
    <div class="main-chat">

      <!-- Chat area content (agent console + messages + input) -->
      <div class="chat-area-scroll">
        <!-- Agent Console (visible when workers exist) -->
        <div class="cline-agent-console" v-if="ai.workers.length > 0">
          <div class="console-header" @click="isConsoleExpanded = !isConsoleExpanded">
            <div class="console-title">
              <span class="console-icon">🤖</span>
              <span>智能开发控制台</span>
              <span class="console-badge" v-if="ai.activeWorkers.length > 0">{{ ai.activeWorkers.length }} 个活跃代理</span>
              <span class="console-badge completed" v-else>运行结束</span>
            </div>
            <div class="console-actions">
              <button v-if="ai.activeWorkers.length > 0" class="console-btn kill-all" @click.stop="handleKillAllWorkers">✕ 全部强杀</button>
              <span class="chevron-icon" :class="{ rotated: isConsoleExpanded }">▼</span>
            </div>
          </div>
          <div class="console-body" v-show="isConsoleExpanded">
            <div v-for="w in ai.workers" :key="w.id" class="agent-card" :class="w.status">
              <div class="agent-card-header" @click="toggleWorkerExpand(w.id)">
                <div class="agent-info">
                  <span class="agent-dot" :style="{ background: statusColors[w.status] }"></span>
                  <span class="agent-role-badge" :title="getWorkerRole(w.taskId)?.description">{{ getWorkerRole(w.taskId)?.icon }} {{ getWorkerRole(w.taskId)?.name }}</span>
                  <span class="agent-name">Worker-{{ w.id.slice(0, 8) }}</span>
                  <span class="agent-task-title" :title="getTaskTitle(w.taskId)">「{{ getTaskTitle(w.taskId) }}」</span>
                  <span class="agent-status-label" :style="{ color: statusColors[w.status] }">{{ statusLabel(w.status) }}</span>
                </div>
                <div class="agent-actions">
                  <span class="agent-stats">Cost: ¥{{ (w.tokenCount * 0.000005).toFixed(4) }} ({{ w.toolCallCount }} 工具)</span>
                  <button v-if="w.status !== 'completed' && w.status !== 'failed' && w.status !== 'killed'" class="agent-kill-btn" @click.stop="handleKillWorker(w.id)">终止</button>
                  <span class="chevron-icon" :class="{ rotated: expandedWorkers[w.id] }">▼</span>
                </div>
              </div>
              <div class="agent-card-body" v-show="expandedWorkers[w.id]">
                <div class="agent-history-log">
                  <div v-for="step in w.history" :key="step.iteration" class="log-step">
                    <div class="step-header"><span class="step-num">#{{ step.iteration }} 轮迭代</span><span class="step-time">{{ formatTime(step.timestamp) }}</span></div>
                    <div class="step-section thought" v-if="step.thought"><div class="section-title">🧠 Thought</div><pre class="section-content">{{ step.thought }}</pre></div>
                    <div class="step-section action" v-if="step.action"><div class="section-title">🔧 Call Tool: <code>{{ step.action.toolName }}</code></div></div>
                    <div class="step-section observation" v-if="step.observation"><div class="section-title">👁️ Observation</div><pre class="section-content">{{ step.observation }}</pre></div>
                  </div>
                  <div class="log-current" v-if="w.currentThought && w.status !== 'completed' && w.status !== 'failed' && w.status !== 'killed'">
                    <div class="step-header"><span class="step-num">正在执行...</span></div>
                    <div class="step-section thought"><pre class="section-content">{{ w.currentThought }}</pre></div>
                  </div>
                  <div class="log-failed-reason" v-if="w.terminationReason">
                    <div class="failed-reason-title">❌ 终止原因</div>
                    <pre class="failed-reason-content">{{ w.terminationReason }}</pre>
                  </div>
                  <div class="log-empty" v-if="(!w.history || w.history.length === 0) && !w.currentThought">暂无迭代历史</div>
                </div>
              </div>
            </div>
            <!-- Task Tree -->
            <div class="console-section" v-if="ai.taskTree.length > 0">
              <div class="console-section-title">📋 任务树 ({{ ai.taskTree.length }})</div>
              <div class="task-tree-compact">
                <div v-for="task in ai.taskTree" :key="task.id" class="task-node-compact">
                  <span :class="['task-status-dot', task.status]"></span>
                  <span class="task-title-compact">{{ task.title }}</span>
                  <span class="task-status-label">{{ statusLabel(task.status) }}</span>
                </div>
              </div>
            </div>
            <!-- KB Search -->
            <div class="console-section" v-if="ai.workers.length > 0">
              <div class="console-section-title"><JcSearchIcon :size="13" /> 知识库</div>
              <div class="kb-search-compact">
                <JcInput beam glow v-model="kbSearchQuery" placeholder="搜索知识库..." style="flex:1;min-width:0" @keyup.enter="searchKnowledgeBase" />
                <JcButton size="small" @click="searchKnowledgeBase">搜索</JcButton>
              </div>
              <div v-for="entry in kbSearchResults" :key="entry.id" class="kb-result-item">
                <div class="kb-result-title">{{ entry.title }}</div>
                <div class="kb-result-preview">{{ entry.content.slice(0, 80) }}...</div>
              </div>
            </div>
          </div>
        </div>

        <!-- Chat Messages -->
        <div class="chat-messages">
          <div v-for="(msg, i) in messages" :key="i" :class="['chat-bubble', msg.role]">
            <div class="bubble-sender">
              <span v-if="msg.roleName" class="bubble-role-badge">{{ msg.roleName }}</span>
              {{ msg.role === 'user' ? '您' : (msg.role === 'system' ? '系统' : (msg.modelName || 'AI Copilot')) }}
            </div>
            <div class="bubble-content" v-html="msg.content.replace(/\n/g, '<br/>')"></div>
          </div>
          <!-- Typing indicator -->
          <div v-if="sending" class="chat-bubble assistant">
            <div class="bubble-sender">{{ aiModel || 'AI' }}</div>
            <div class="bubble-content typing"><span class="dot"></span><span class="dot"></span><span class="dot"></span></div>
          </div>
        </div>

        <!-- Shortcut Pills -->
        <Transition name="fade-slide">
          <div class="shortcut-pills" v-if="hasContent">
            <span class="shortcut-pill-desc">针对当前输入：</span>
            <button class="shortcut-pill" @click="polishMemo" :disabled="sending">✨ 润色排版</button>
            <button class="shortcut-pill" @click="recommendTags" :disabled="sending">🏷️ 提取标签</button>
          </div>
        </Transition>

        <!-- ═══ Input Card (DeepSeek style + Mode Selector) ═══ -->
        <div class="ds-input-card" :class="{ focused: isFocused, 'has-content': hasContent }">
          <!-- Mode Selector: 执行 / 问答 / 规划 -->
          <div class="mode-chip-row">
            <div class="mode-selector">
              <button v-for="mode in chatModes" :key="mode"
                :class="['mode-btn', { active: chatMode === mode }]"
                @click="chatMode = mode"
                :title="mode === '创作' ? '读写执行，调用全部工具' : mode === '问答' ? '只读问答，不执行操作' : '多级任务拆解，P0-P4 优先级规划'">
                {{ mode }}
              </button>
            </div>

            <!-- Role selector (only in 问答 mode) -->
            <div v-if="chatMode === '问答'" class="ds-pill-select-wrap">
              <select v-model="activeChatRoleId" class="ds-pill-select" style="max-width:92px" title="切换当前对话角色">
                <option value="auto">智能路由</option>
                <option v-for="r in chatRolesList" :key="r.id" :value="r.id">{{ r.icon }} {{ r.name }}</option>
              </select>
            </div>
            <div v-if="chatMode === '创作'" class="ds-pill-select-wrap agent-label">
              <span class="ds-pill-text">智能体团队 (多角色协同)</span>
            </div>
            <div v-if="chatMode === '规划'" class="ds-pill-select-wrap agent-label plan-label">
              <span class="ds-pill-text">任务规划模式</span>
            </div>

            <span class="chip-spacer"></span>

            <!-- Deep Think -->
            <div class="ds-pill-select-wrap" :class="{ active: enableDeepThink }">
              <button class="ds-pill-inline-btn" @click="enableDeepThink = !enableDeepThink" title="深度思考：展示思维链过程">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                  <ellipse cx="12" cy="12" rx="3" ry="9" transform="rotate(45 12 12)"></ellipse>
                  <ellipse cx="12" cy="12" rx="3" ry="9" transform="rotate(-45 12 12)"></ellipse>
                  <circle cx="12" cy="12" r="1.5" fill="currentColor"></circle>
                </svg>
                深度思考
              </button>
            </div>

            <!-- Local KB -->
            <div class="ds-pill-select-wrap" :class="{ active: enableLocalKb }">
              <button class="ds-pill-inline-btn" @click="enableLocalKb = !enableLocalKb" title="本地知识库：检索笔记作为参考">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                  <circle cx="12" cy="12" r="9"></circle>
                  <line x1="2" y1="12" x2="22" y2="12"></line>
                  <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path>
                </svg>
                知识库
              </button>
            </div>

            <!-- Model selector -->
            <div class="ds-pill-select-wrap">
              <svg class="model-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                <rect x="3" y="11" width="18" height="10" rx="2"></rect>
                <circle cx="12" cy="5" r="2"></circle>
                <path d="M12 7v4"></path>
              </svg>
              <select v-model="selectedCombinedModel" @change="handleModelChange" class="ds-pill-select" title="切换 AI 模型">
                <optgroup v-for="(models, providerName) in modelOptions" :key="providerName" :label="providerName">
                  <option v-for="m in models" :key="m.name" :value="m.id">{{ m.label }}</option>
                </optgroup>
              </select>
              <button v-if="aiProvider === 'vllm' || aiProvider === 'ollama'" class="ds-pill-refresh" @click="refreshLocalModels" :disabled="loadingModels" title="刷新本地模型列表">
                <svg viewBox="0 0 16 16" class="refresh-icon-svg" :class="{ spinning: loadingModels }" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M1.5 8a6.5 6.5 0 0 1 10.5-5L14 5m0-3.5V5h-3.5M14.5 8a6.5 6.5 0 0 1-10.5 5L2 11m0 3.5V11h3.5"></path>
                </svg>
              </button>
              
            </div>

            <!-- Workspace selector -->
            <div class="ds-pill-select-wrap workspace">
              <button class="ds-pill-inline-btn workspace-btn" @click="handleSelectWorkspace" :title="ai.workspaceRoot || '选择工作区'">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" style="width:13px;height:13px;flex-shrink:0">
                  <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
                </svg>
                {{ workspaceShortName }}
              </button>
            </div>
          </div>

          <!-- Textarea -->
          <textarea ref="inputTextarea" v-model="userInput" :placeholder="placeholderText"
            class="ds-textarea"
            @focus="isFocused = true"
            @blur="isFocused = false"
            @keydown.enter.prevent="handleEnterKey"
            @input="autoResizeTextarea"
          ></textarea>

          <!-- Control Bar -->
          <div class="ds-control-bar">
            <div class="ds-pills">
              <button v-if="notesStore.activeNoteTabId" class="ds-action-btn attach" @click="attachActiveNote" title="附件当前笔记">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21.44 11.05l-9.19 9.19a6 6 0 0 1-8.49-8.49l9.19-9.19a4 4 0 0 1 5.66 5.66l-9.2 9.19a2 2 0 0 1-2.83-2.83l8.49-8.48"></path>
                </svg>
              </button>
            </div>
            <div class="ds-actions">
              <button class="ds-action-btn clear" @click="clearChat" title="清空对话">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.1" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="3 6 5 6 21 6"></polyline>
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                </svg>
              </button>
              <button class="ds-send-btn" :disabled="sending || !userInput.trim()" @click="sendMessage">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="12" y1="19" x2="12" y2="5"></line>
                  <polyline points="5 12 12 5 19 12"></polyline>
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ═══ Delete Confirm Dialog ═══ -->
    <JcModal v-model:open="confirmDelete.show" title="删除对话" width="420">
      <div class="confirm-desc">确定要删除对话「<strong>{{ confirmDelete.title }}</strong>」吗？此操作不可撤销，对话消息将从本地永久删除。</div>
      <template #footer>
        <button class="confirm-btn cancel" @click="confirmDelete.show = false">取消</button>
        <button class="confirm-btn delete" @click="executeDeleteSession">删除</button>
      </template>
    </JcModal>

    <!-- ═══ Browser Dialog ═══ -->
    <JcModal v-model:open="showBrowserDialog" title="🌐 打开浏览器" width="440">
      <div class="browser-modal-body">
        <input v-model="browserUrlInput" class="browser-url-input" placeholder="输入 URL..." @keyup.enter="handleBrowserConfirm" @click.stop />
      </div>
      <template #footer>
        <button class="footer-btn-cancel" @click="showBrowserDialog = false">取消</button>
        <button class="footer-btn-save" @click="handleBrowserConfirm">打开</button>
      </template>
    </JcModal>

    <!-- ═══ Approval Overlay ═══ -->
    <div class="overlay-backdrop" v-if="ai.pendingApprovals.length > 0">
      <div class="overlay-card">
        <div class="overlay-header">
          <h3 class="overlay-title">🛡️ 聚合安全审批队列</h3>
          <span class="overlay-badge">{{ ai.pendingApprovals.length }} 项敏感操作</span>
        </div>
        <div class="overlay-body">
          <div v-for="req in ai.pendingApprovals" :key="req.id" class="approval-card-item">
            <div class="approval-card-meta">
              <span class="risk-badge" :style="{ background: riskColors[req.riskLevel] }">{{ req.riskLevel }}</span>
              <span class="approval-tool">{{ req.toolName }}</span>
              <span class="approval-worker">Worker: {{ req.workerId.slice(0, 8) }}</span>
            </div>
            <div class="approval-card-reason">{{ req.reason }}</div>
            <div class="approval-card-args" v-if="req.arguments && Object.keys(req.arguments).length > 0">
              <strong>参数:</strong> <code>{{ JSON.stringify(req.arguments) }}</code>
            </div>
            <pre class="approval-card-diff" v-if="req.diffPreview">{{ req.diffPreview }}</pre>
            <div class="approval-card-actions">
              <button class="btn btn-sm btn-success" @click="handleApprove(req)">✓ 批准执行</button>
              <button class="btn btn-sm btn-danger" @click="handleDeny(req)">✗ 拒绝</button>
            </div>
          </div>
        </div>
        <div class="overlay-footer">
          <button class="btn btn-danger" @click="handleDenyAll">✗ 一键全部拒绝</button>
          <button class="btn btn-success" @click="handleApproveAll">✓ 一键全部批准</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
/* ═══ Sidebar + Main Layout ═══ */
.ai-helper-container {
  display: flex; flex: 1; height: 100%;
  overflow: hidden; background: var(--jc-bg-app);
}

/* ── Sidebar ── */
.sidebar {
  width: 250px; min-width: 250px; height: 100%;
  background: var(--jc-bg-elevated); display: flex; flex-direction: column;
  border-right: 1px solid var(--jc-border-default); overflow: hidden;
}
.sidebar-header {
  display: flex; align-items: center; gap: 6px; padding: 14px 12px 10px;
}
.sidebar-logo { flex-shrink: 0; color: var(--jc-color-accent); }
.logo-text { font-size: 15px; font-weight: 700; color: var(--jc-text-highlight); letter-spacing: -0.2px; }
.sidebar-subtitle { font-size: 11px; color: var(--jc-text-secondary); font-weight: 500; }
.header-spacer { flex: 1; }
.header-icon-btn {
  width: 26px; height: 26px; display: flex; align-items: center; justify-content: center;
  background: transparent; border: none; border-radius: 6px;
  color: var(--jc-text-secondary); cursor: pointer; transition: all 0.12s;
  &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
}

.new-chat-btn {
  display: flex; align-items: center; justify-content: center; gap: 6px;
  width: calc(100% - 24px); margin: 0 12px 10px; height: 40px;
  background: var(--jc-color-accent); color: #fff; border: none; border-radius: 8px;
  font-size: 12px; font-weight: 600; cursor: pointer; font-family: inherit; transition: all 0.15s;
  &:hover { opacity: 0.9; transform: translateY(-1px); box-shadow: 0 2px 8px rgba(0,0,0,0.2); }
  &:active { transform: translateY(0); }
}

.session-list {
  flex: 1; overflow-y: auto; padding: 0 6px; display: flex; flex-direction: column; gap: 1px;
}
.session-item-wrap {
  display: flex; align-items: center; gap: 0; border-radius: 6px;
  transition: all 0.12s;
  &:hover { background: var(--jc-bg-hover); }
  &.active { background: var(--jc-bg-selected); }
}
.session-item {
  display: flex; align-items: center; gap: 6px; flex: 1; min-width: 0; padding: 8px 6px 8px 10px;
  background: transparent; border: none; border-radius: 6px 0 0 6px; cursor: pointer;
  font-family: inherit; text-align: left; transition: all 0.12s;
  .sess-icon { color: var(--jc-text-secondary); flex-shrink: 0; }
  .sess-title {
    flex: 1; font-size: 12px; color: var(--jc-text-primary); overflow: hidden;
    text-overflow: ellipsis; white-space: nowrap; min-width: 0;
  }
  .sess-date { font-size: 10px; color: var(--jc-text-secondary); flex-shrink: 0; opacity: 0.7; }
}
.session-del-btn {
  display: none; align-items: center; justify-content: center;
  width: 22px; height: 22px; margin-right: 6px; flex-shrink: 0;
  background: transparent; border: none; border-radius: 4px;
  color: var(--jc-text-secondary); font-size: 10px; cursor: pointer;
  transition: all 0.12s;
  .session-item-wrap:hover & { display: flex; }
  &:hover { background: rgba(248,81,73,0.15); color: #f85149; }
}
.session-empty { text-align: center; padding: 20px 12px; font-size: 12px; color: var(--jc-text-secondary); }

.sidebar-footer {
  padding: 6px 8px; border-top: 1px solid var(--jc-border-default);
  display: flex; flex-direction: row; gap: 8px;
}
.sidebar-footer-btn {
  display: flex; align-items: center; justify-content: center; gap: 4px;
  flex: 1; padding: 5px 4px;
  background: transparent; border: none; border-radius: 6px; cursor: pointer;
  font-family: inherit; font-size: 10.5px; color: var(--jc-text-secondary);
  transition: all 0.12s;
  &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
  span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
}

/* ── Main Chat ── */
.main-chat { flex: 1; display: flex; flex-direction: column; min-width: 0; background: var(--jc-bg-app); }

.chat-area-scroll {
  flex: 1; display: flex; flex-direction: column; min-height: 0; padding: 2px; gap: 2px; overflow: hidden;
}

/* ── Agent Console ── */
.cline-agent-console {
  background: var(--jc-bg-panel); border: 1px solid var(--jc-border-default);
  border-radius: 8px; overflow: hidden; margin-bottom: 4px;
  display: flex; flex-direction: column; max-height: 280px; flex-shrink: 0;

  .console-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; background: var(--jc-bg-input); cursor: pointer;
    border-bottom: 1px solid var(--jc-border-default); user-select: none;
    &:hover { background: var(--jc-bg-hover); }
  }
  .console-title {
    display: flex; align-items: center; gap: 8px; font-size: 12px; font-weight: 600; color: var(--jc-text-highlight);
    .console-icon { font-size: 14px; }
    .console-badge {
      font-size: 10px; padding: 1px 6px; border-radius: 10px;
      background: rgba(138, 88, 255, 0.15); color: var(--jc-color-accent); font-weight: 500;
      &.completed { background: rgba(63, 185, 80, 0.15); color: #3fb950; }
    }
  }
  .console-actions {
    display: flex; align-items: center; gap: 10px;
    .console-btn.kill-all {
      background: rgba(248, 81, 73, 0.1); border: 1px solid rgba(248, 81, 73, 0.3);
      color: #f85149; font-size: 10px; padding: 2px 6px; border-radius: 4px; cursor: pointer; font-weight: 500;
      &:hover { background: #f85149; color: #fff; }
    }
  }
  .chevron-icon { font-size: 8px; color: var(--jc-text-secondary); transition: transform 0.2s; &.rotated { transform: rotate(180deg); } }
  .console-body {
    overflow-y: auto; padding: 6px; display: flex; flex-direction: column; gap: 6px; background: var(--jc-bg-panel);
    &::-webkit-scrollbar { width: 4px; }
    &::-webkit-scrollbar-thumb { background: var(--jc-border-default); border-radius: 2px; }
  }
  .agent-card {
    border: 1px solid var(--jc-border-default); border-radius: 6px; overflow: hidden; background: var(--jc-bg-input);
    &.thinking { border-left: 3px solid #58a6ff; }
    &.acting { border-left: 3px solid #f0883e; }
    &.observing { border-left: 3px solid #a371f7; }
    &.waitingApproval { border-left: 3px solid #d29922; }
    &.completed { border-left: 3px solid #3fb950; opacity: 0.85; }
    &.failed { border-left: 3px solid #f85149; }
    &.killed { border-left: 3px solid #8b949e; opacity: 0.75; }
    .agent-card-header {
      display: flex; justify-content: space-between; align-items: center; padding: 6px 10px; cursor: pointer; font-size: 11px; user-select: none;
      &:hover { background: var(--jc-bg-hover); }
    }
    .agent-info {
      display: flex; align-items: center; gap: 6px; min-width: 0;
      .agent-dot { width: 6px; height: 6px; border-radius: 50%; flex-shrink: 0; }
      .agent-role-badge { font-size: 10px; padding: 1px 6px; background: rgba(138, 88, 255, 0.1); border: 1px solid rgba(138, 88, 255, 0.2); color: var(--jc-color-accent); border-radius: 4px; font-weight: 500; white-space: nowrap; }
      .agent-name { font-weight: 600; color: var(--jc-text-primary); font-family: monospace; }
      .agent-task-title { color: var(--jc-text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 150px; }
      .agent-status-label { font-weight: 500; font-size: 10px; }
    }
    .agent-actions {
      display: flex; align-items: center; gap: 8px;
      .agent-stats { font-size: 10px; color: var(--jc-text-secondary); }
      .agent-kill-btn { background: transparent; border: 1px solid var(--jc-border-default); color: var(--jc-text-secondary); font-size: 9px; padding: 1px 4px; border-radius: 3px; cursor: pointer; &:hover { background: rgba(248, 81, 73, 0.1); border-color: #f85149; color: #f85149; } }
    }
    .agent-card-body { border-top: 1px solid var(--jc-border-default); background: var(--jc-bg-panel); padding: 8px; }
    .agent-history-log { display: flex; flex-direction: column; gap: 10px; max-height: 200px; overflow-y: auto; font-family: monospace; font-size: 10.5px; padding-right: 4px; }
    .log-step { border-bottom: 1px dashed var(--jc-border-default); padding-bottom: 8px; &:last-child { border-bottom: none; padding-bottom: 0; } }
    .step-header { display: flex; justify-content: space-between; color: var(--jc-text-secondary); margin-bottom: 4px; font-size: 10px; .step-num { font-weight: bold; color: var(--jc-color-accent); } }
    .step-section { margin-top: 4px; display: flex; flex-direction: column; gap: 2px; .section-title { font-weight: 600; color: var(--jc-text-primary); font-size: 10.5px; } .section-content { margin: 0; padding: 4px 6px; background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); border-radius: 4px; white-space: pre-wrap; word-break: break-all; color: var(--jc-text-primary); max-height: 100px; overflow-y: auto; } }
    .log-current { padding-top: 4px; .step-num { color: #58a6ff; animation: pulse 1.5s infinite; } .section-content { border-left: 2px solid #58a6ff; } }
    .log-failed-reason { margin-top: 6px; padding: 6px; background: rgba(248, 81, 73, 0.05); border: 1px solid rgba(248, 81, 73, 0.2); border-radius: 4px; .failed-reason-title { font-weight: bold; color: #f85149; margin-bottom: 2px; } .failed-reason-content { margin: 0; white-space: pre-wrap; color: var(--jc-text-primary); } }
    .log-empty { text-align: center; color: var(--jc-text-secondary); padding: 12px; }
  }
}

@keyframes pulse { 0% { opacity: 0.6; } 50% { opacity: 1; } 100% { opacity: 0.6; } }

/* ── Chat Messages ── */
.chat-messages {
  flex: 1; overflow-y: auto; border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel); border-radius: 2px; padding: 4px;
  display: flex; flex-direction: column; gap: 6px;
  &::-webkit-scrollbar { width: 4px; }
  &::-webkit-scrollbar-thumb { background: var(--jc-border-default); border-radius: 2px; }
}

.chat-bubble {
  max-width: 85%; display: flex; flex-direction: column; gap: 4px; align-self: flex-start;
  .bubble-sender { font-size: 11px; color: var(--jc-text-secondary); font-weight: 500; display: flex; align-items: center; }
  .bubble-role-badge { font-size: 10px; padding: 1px 6px; background: rgba(138, 88, 255, 0.12); border: 1px solid rgba(138, 88, 255, 0.25); color: var(--jc-color-accent); border-radius: 4px; font-weight: bold; margin-right: 4px; }
  .bubble-content { background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); color: var(--jc-text-primary); font-size: 13px; line-height: 1.6; padding: 10px 14px; border-radius: 8px; word-break: break-word; }
  &.user { align-self: flex-end; align-items: flex-end; .bubble-content { background: var(--jc-bg-selected); border-color: var(--jc-color-accent); } }
  &.system { .bubble-sender { color: #f0883e; } .bubble-content { border-left: 3px solid #f0883e; font-size: 12px; opacity: 0.9; } }
  .bubble-content.typing { display: flex; gap: 4px; padding: 4px 2px; align-items: center; }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: var(--jc-text-secondary); animation: typingBounce 1.4s infinite ease-in-out both; &:nth-child(1) { animation-delay: -0.32s; } &:nth-child(2) { animation-delay: -0.16s; } &:nth-child(3) { animation-delay: 0s; } }
}

@keyframes typingBounce { 0%,80%,100% { transform: scale(0.6); opacity: 0.4; } 40% { transform: scale(1); opacity: 1; } }

/* ── Shortcut Pills ── */
.shortcut-pills {
  display: flex; align-items: center; gap: 8px; margin-bottom: 2px; flex-wrap: wrap; padding: 0 4px;
  .shortcut-pill-desc { font-size: 11px; color: var(--jc-text-secondary); font-weight: 500; }
  .shortcut-pill { background: var(--jc-bg-btn); border: 1px solid var(--jc-border-default); color: var(--jc-text-primary); font-size: 11px; padding: 4px 10px; border-radius: 20px; cursor: pointer; font-weight: 500; transition: all 0.2s; &:hover:not(:disabled) { background: var(--jc-bg-hover); border-color: var(--jc-color-accent); color: var(--jc-color-accent); } &:disabled { opacity: 0.5; cursor: not-allowed; } }
}
.fade-slide-enter-active, .fade-slide-leave-active { transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1); }
.fade-slide-enter-from, .fade-slide-leave-to { opacity: 0; transform: translateY(6px) scale(0.98); }

/* ── Mode Chip Row ── */
.mode-chip-row {
  display: flex; gap: 6px; align-items: center; flex-wrap: wrap;
}
.chip-spacer { flex: 1; min-width: 4px; }

/* Mode selector: 执行 / 问答 / 规划 */
.mode-selector {
  display: flex; gap: 2px; height: 28px;
  background: var(--jc-bg-panel); border: 1px solid var(--jc-border-default); border-radius: 20px; padding: 2px; flex-shrink: 0;
}
.mode-btn {
  padding: 0 12px; border: none; border-radius: 16px; background: transparent;
  color: var(--jc-text-secondary); font-size: 11px; font-weight: 600; cursor: pointer;
  font-family: inherit; white-space: nowrap; transition: all 0.12s;
  &:hover { color: var(--jc-text-primary); }
  &.active { background: var(--jc-bg-elevated); color: var(--jc-color-accent); box-shadow: 0 1px 3px rgba(0,0,0,0.08); }
}

/* ── Input Card ── */
.ds-input-card {
  display: flex; flex-direction: column; background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-default); border-radius: 4px; padding: 8px 14px; gap: 8px;
  transition: all 0.25s cubic-bezier(0.4, 0, 0.2, 1); box-shadow: 0 4px 16px rgba(0, 0, 0, 0.05);
  &:hover { border-color: var(--jc-border-strong); }
  &.focused { border-color: var(--jc-color-accent, #8a58ff); box-shadow: 0 0 12px rgba(138, 88, 255, 0.15); }

  .ds-textarea {
    width: 100%; min-height: 44px; max-height: 200px; background: transparent;
    border: none; resize: none; outline: none; color: var(--jc-text-primary);
    font-size: 13.5px; font-family: inherit; line-height: 1.6; padding: 2px 0; overflow-y: auto;
  }

  .ds-control-bar { display: flex; justify-content: space-between; align-items: center; }
  .ds-pills { display: flex; gap: 8px; align-items: center; }

  .ds-pill-select-wrap {
    display: inline-flex; align-items: center; gap: 3px; padding: 0 6px; height: 22px;
    color: var(--jc-text-secondary); position: relative; transition: all 0.15s;
    &:hover { color: var(--jc-text-primary); }
    &.active { border-color: var(--jc-color-accent, #8a58ff); color: var(--jc-color-accent, #8a58ff); }
    &.workspace { max-width: 130px; overflow: hidden; }
    &.agent-label { border: 1px solid rgba(138, 88, 255, 0.3); border-radius: 4px; padding: 0 8px; height: 22px; }
    &.plan-label { border-color: rgba(88, 166, 255, 0.3); .ds-pill-text { color: #58a6ff; } }

    .ds-pill-select {
      background: transparent; border: none; outline: none; color: inherit;
      font-size: 11.5px; font-weight: 500; cursor: pointer; padding: 0 14px 0 0; max-width: 120px;
      appearance: none;
      background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='8' height='8' viewBox='0 0 24 24' fill='none' stroke='gray' stroke-width='3' stroke-linecap='round' stroke-linejoin='round'><polyline points='6 9 12 15 18 9'></polyline></svg>");
      background-repeat: no-repeat; background-position: right center; background-size: 8px;
      optgroup, option { background: var(--jc-bg-panel); color: var(--jc-text-primary); }
    }
    .ds-pill-refresh {
      background: transparent; border: none; outline: none; cursor: pointer;
      display: inline-flex; align-items: center; padding: 0 0 0 6px; margin-left: 2px;
      border-left: 1px solid var(--jc-border-default); color: var(--jc-text-secondary); height: 14px;
      .refresh-icon-svg { width: 11px; height: 11px; &.spinning { animation: spin-anim 1s linear infinite; } }
    }
    .ds-pill-settings-btn {
      background: transparent; border: none; outline: none; cursor: pointer;
      display: inline-flex; align-items: center; padding: 0 0 0 4px; margin-left: 2px;
      border-left: 1px solid var(--jc-border-default); color: var(--jc-text-secondary); height: 14px;
    }
  }

  .ds-pill-inline-btn {
    display: inline-flex; align-items: center; gap: 3px;
    background: transparent; border: none; color: inherit;
    font-size: 10.5px; font-weight: 500; cursor: pointer; padding: 0; white-space: nowrap; outline: none;
  }
  .workspace-btn { max-width: 100px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .ds-actions { display: flex; align-items: center; gap: 8px; }
  .ds-action-btn {
    display: flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; background: transparent; border: none; border-radius: 6px;
    color: var(--jc-text-secondary); cursor: pointer;
    svg { width: 17px; height: 17px; }
    &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); }
    &.clear:hover { color: var(--jc-color-error); background: rgba(220, 38, 38, 0.08); }
    &.attach:hover { color: var(--jc-color-success); background: rgba(46, 204, 113, 0.08); }
  }

  .ds-send-btn {
    display: flex; align-items: center; justify-content: center;
    width: 28px; height: 28px; border-radius: 50%; border: none;
    background: var(--jc-border-default); color: var(--jc-bg-app); cursor: not-allowed;
    svg { width: 16px; height: 16px; }
    &:not(:disabled) { background: var(--jc-color-accent, #8a58ff); color: #ffffff; cursor: pointer; &:hover { transform: scale(1.06); } }
  }
}

/* ── Confirm Modal ── */
.confirm-modal { background: var(--jc-bg-app); border: 1px solid var(--jc-border-default); border-radius: 10px; width: 360px; padding: 24px 20px 16px; display: flex; flex-direction: column; align-items: center; gap: 10px; box-shadow: 0 12px 40px rgba(0,0,0,0.4); }
.confirm-icon { font-size: 32px; }
.confirm-title { font-size: 15px; font-weight: 700; color: var(--jc-text-highlight); }
.confirm-desc { font-size: 12px; color: var(--jc-text-secondary); text-align: center; line-height: 1.5; padding: 0 4px; }
.confirm-actions { display: flex; gap: 10px; margin-top: 6px; width: 100%; }
.confirm-btn { flex: 1; padding: 8px 0; border: none; border-radius: 6px; font-size: 12px; font-weight: 600; cursor: pointer; font-family: inherit; transition: all 0.12s; }
.confirm-btn.cancel { background: var(--jc-bg-panel); color: var(--jc-text-secondary); &:hover { background: var(--jc-bg-hover); color: var(--jc-text-primary); } }
.confirm-btn.delete { background: #da3633; color: #fff; &:hover { background: #f85149; } }

/* ── Overlay / Browser Modal ── */
.session-overlay { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); z-index: 9999; display: flex; align-items: center; justify-content: center; padding: 20px; }
.session-modal-header { display: flex; align-items: center; justify-content: space-between; padding: 14px 16px; border-bottom: 1px solid var(--jc-border-default); font-weight: 600; font-size: 14px; }
.session-modal-close { background: transparent; border: none; color: var(--jc-text-secondary); cursor: pointer; font-size: 16px; &:hover { color: var(--jc-text-primary); } }
.browser-modal { background: var(--jc-bg-app); border: 1px solid var(--jc-border-default); border-radius: 8px; width: 400px; max-width: 90vw; box-shadow: 0 12px 40px rgba(0,0,0,0.4); }
.browser-modal-body { display: flex; gap: 8px; padding: 12px 16px 16px; }
.browser-url-input { flex: 1; background: var(--jc-bg-input); border: 1px solid var(--jc-border-default); border-radius: 6px; padding: 8px 12px; color: var(--jc-text-primary); font-size: 13px; outline: none; &:focus { border-color: var(--jc-color-accent); } }
.browser-go-btn { background: var(--jc-color-accent); color: #fff; border: none; border-radius: 6px; padding: 8px 16px; font-size: 13px; cursor: pointer; white-space: nowrap; &:hover { opacity: 0.85; } }

/* ── Approval Overlay ── */
.overlay-backdrop { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.55); backdrop-filter: blur(10px); z-index: 9999; display: flex; align-items: center; justify-content: center; padding: 20px; }
.overlay-card { width: 100%; max-width: 580px; max-height: 85vh; background: var(--jc-bg-app); border: 1px solid var(--jc-border-default); border-radius: 12px; box-shadow: 0 12px 40px rgba(0,0,0,0.4); display: flex; flex-direction: column; overflow: hidden; }
.overlay-header { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid var(--jc-border-default); }
.overlay-title { font-size: 15px; font-weight: 700; margin: 0; color: #f0883e; }
.overlay-badge { font-size: 11px; font-weight: 600; padding: 2px 8px; border-radius: 10px; background: rgba(248,81,73,0.15); color: #f85149; }
.overlay-body { flex: 1; overflow-y: auto; padding: 18px; display: flex; flex-direction: column; gap: 14px; }
.approval-card-item { border: 1px solid var(--jc-border-default); border-radius: 8px; padding: 12px; }
.approval-card-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.risk-badge { padding: 2px 7px; border-radius: 8px; font-size: 9px; font-weight: 700; color: #fff; text-transform: uppercase; }
.approval-tool { flex: 1; font-weight: 600; font-size: 13px; }
.approval-worker { font-family: monospace; font-size: 11px; color: var(--jc-text-secondary); }
.approval-card-reason { font-size: 12px; color: var(--jc-text-secondary); line-height: 1.4; }
.approval-card-args { font-size: 11px; margin-top: 6px; background: rgba(0,0,0,0.1); padding: 4px 8px; border-radius: 4px; word-break: break-all; }
.approval-card-diff { margin-top: 8px; padding: 8px; background: #0c1117; border: 1px solid var(--jc-border-default); border-radius: 6px; font-size: 11px; font-family: monospace; overflow-x: auto; max-height: 180px; color: #c9d1d9; white-space: pre-wrap; line-height: 1.4; }
.approval-card-actions { display: flex; gap: 8px; margin-top: 10px; justify-content: flex-end; }
.overlay-footer { padding: 14px 18px; border-top: 1px solid var(--jc-border-default); display: flex; justify-content: space-between; gap: 12px; }

.btn { padding: 6px 14px; border: 1px solid var(--jc-border-default); border-radius: 6px; background: var(--jc-bg-elevated); color: var(--jc-text-primary); font-size: 13px; cursor: pointer; font-weight: 500; &:hover { background: var(--jc-bg-hover); } }
.btn-success { background: #238636; color: white; border-color: #2ea043; &:hover { background: #2ea043; } }
.btn-danger { background: #da3633; color: white; border-color: #f85149; &:hover { background: #f85149; } }
.btn-sm { padding: 3px 10px; font-size: 12px; }

@keyframes spin-anim { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
</style>
