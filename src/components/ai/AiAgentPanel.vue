<script setup lang="ts">
import { onMounted, onUnmounted, ref, computed } from 'vue'
import { useAiStore } from '@/stores/ai'
import type { TaskNode, WorkerState, ApprovalRequest } from '@/types/ai'

const ai = useAiStore()

const newSessionTitle = ref('')
const taskRequest = ref('')
const selectedTaskId = ref<string | null>(null)
const systemPrompt = ref('你是一个专业的代码助手，请使用 ReAct 模式完成任务。')
const pollTimer = ref<number | null>(null)
const manualPath = ref('')

const showSettings = ref(false)
const localConfig = ref({
  inputCachedCostPerM: 0.025,
  inputUncachedCostPerM: 3.0,
  outputCostPerM: 6.0,
  costLimit: 5.0
})

const selectedTask = computed<TaskNode | null>(() =>
  ai.taskTree.find((t) => t.id === selectedTaskId.value) ?? null,
)

const statusColors: Record<string, string> = {
  thinking: '#58a6ff',
  acting: '#f0883e',
  observing: '#a371f7',
  waitingApproval: '#d29922',
  completed: '#3fb950',
  failed: '#f85149',
  killed: '#8b949e',
  pending: '#8b949e',
  inProgress: '#58a6ff',
  blocked: '#d29922',
  active: '#3fb950',
  paused: '#8b949e',
}

const riskColors: Record<string, string> = {
  low: '#3fb950',
  medium: '#d29922',
  high: '#f0883e',
  critical: '#f85149',
}

function statusLabel(s: string): string {
  const map: Record<string, string> = {
    thinking: '思考中', acting: '执行中', observing: '观察中',
    waitingApproval: '待审批', completed: '已完成', failed: '失败',
    killed: '已终止', pending: '待处理', inProgress: '进行中',
    blocked: '阻塞', active: '活跃', paused: '暂停',
  }
  return map[s] ?? s
}

async function handleCreateSession() {
  if (!newSessionTitle.value.trim()) return
  await ai.createSession(newSessionTitle.value.trim())
  newSessionTitle.value = ''
}

async function handlePlanTask() {
  if (!ai.currentSessionId || !taskRequest.value.trim()) return
  await ai.planTask(ai.currentSessionId, taskRequest.value.trim())
  taskRequest.value = ''
}

async function handleSpawnWorker(task: TaskNode) {
  if (!ai.currentSessionId) return
  await ai.spawnWorker(ai.currentSessionId, task, systemPrompt.value)
}

async function handleApprove(req: ApprovalRequest) {
  await ai.approveRequest(req.id)
}

async function handleDeny(req: ApprovalRequest) {
  await ai.denyRequest(req.id)
}

async function handleApproveAll() {
  for (const req of ai.pendingApprovals) {
    await ai.approveRequest(req.id)
  }
}

async function handleDenyAll() {
  for (const req of ai.pendingApprovals) {
    await ai.denyRequest(req.id)
  }
}

async function handlePromote(entryId: string) {
  await ai.promoteKnowledge(entryId)
}

async function saveSettings() {
  await ai.updateCostConfig(localConfig.value)
  showSettings.value = false
}

function estimateCost(tokenCount: number): string {
  const avgRate = (ai.costConfig.inputUncachedCostPerM * 0.8) + (ai.costConfig.outputCostPerM * 0.2);
  const cost = (tokenCount * avgRate) / 1_000_000.0;
  return cost.toFixed(4);
}

async function handleKillWorker(workerId: string) {
  await ai.killWorker(workerId)
}

async function handleSelectWorkspace() {
  await ai.changeWorkspaceDialog()
  manualPath.value = ai.workspaceRoot
}

async function handleManualWorkspace() {
  if (!manualPath.value.trim()) return
  await ai.changeWorkspaceManual(manualPath.value.trim())
}

function startPolling() {
  pollTimer.value = window.setInterval(async () => {
    await Promise.all([
      ai.loadWorkers(), 
      ai.loadPendingApprovals(),
      ai.loadDrafts()
    ])
  }, 2000)
}

function stopPolling() {
  if (pollTimer.value !== null) {
    clearInterval(pollTimer.value)
    pollTimer.value = null
  }
}

onMounted(async () => {
  await ai.loadSessions()
  await ai.loadWorkspaceRoot()
  manualPath.value = ai.workspaceRoot
  await ai.initListeners()
  await ai.loadDrafts()
  localConfig.value = { ...ai.costConfig }
  startPolling()
})

onUnmounted(() => {
  stopPolling()
  ai.destroyListeners()
})
</script>

<template>
  <div class="ai-panel">
    <!-- Header -->
    <div class="ai-header">
      <h2 class="ai-title">🤖 JC9 AI Coding Agent</h2>
      <div class="ai-stats">
        <span class="stat-badge" title="活跃 Worker">⚡ {{ ai.activeWorkers.length }}</span>
        <span class="stat-badge warn" title="待审批" v-if="ai.pendingApprovalsCount > 0">⚠️ {{ ai.pendingApprovalsCount }}</span>
      </div>
    </div>

    <!-- 工作空间管理 -->
    <div class="workspace-card">
      <div class="workspace-meta">
        <span class="workspace-label">📁 工作区根目录:</span>
        <span class="workspace-path" :title="ai.workspaceRoot">{{ ai.workspaceRoot || '正在获取工作区...' }}</span>
      </div>
      <div class="workspace-controls">
        <button class="btn btn-sm btn-primary" @click="handleSelectWorkspace">选择文件夹</button>
        <div class="workspace-manual">
          <input v-model="manualPath" placeholder="手动输入绝对路径..." class="input input-inline-sm" @keyup.enter="handleManualWorkspace" />
          <button class="btn btn-sm" @click="handleManualWorkspace">指定</button>
        </div>
      </div>
    </div>

    <!-- 计费与限额设置 (参考 DS 定价) -->
    <div class="section settings-section">
      <div class="section-title font-gold" @click="showSettings = !showSettings" style="cursor: pointer; display: flex; align-items: center; justify-content: space-between; width: 100%;">
        <span>⚙️ 计费与防爆限额设置 (DeepSeek 定价)</span>
        <span style="font-size: 11px; color: var(--jc-text-secondary);">{{ showSettings ? '折叠 ▲' : '展开 ▼' }}</span>
      </div>
      <div class="settings-body" v-show="showSettings">
        <div class="settings-row">
          <label>缓存未命中输入 (元/百万 Token):</label>
          <input type="number" step="0.1" v-model="localConfig.inputUncachedCostPerM" class="input input-inline" />
        </div>
        <div class="settings-row">
          <label>缓存命中输入 (元/百万 Token):</label>
          <input type="number" step="0.001" v-model="localConfig.inputCachedCostPerM" class="input input-inline" />
        </div>
        <div class="settings-row">
          <label>输出 Token 价格 (元/百万 Token):</label>
          <input type="number" step="0.1" v-model="localConfig.outputCostPerM" class="input input-inline" />
        </div>
        <div class="settings-row">
          <label>防爆熔断限额 (元 ¥):</label>
          <input type="number" step="0.5" v-model="localConfig.costLimit" class="input input-inline" />
        </div>
        <div class="settings-actions">
          <button class="btn btn-sm btn-primary" @click="saveSettings">保存配置</button>
        </div>
      </div>
    </div>

    <!-- 会话管理 -->
    <div class="section">
      <div class="section-title font-gold">会话管理</div>
      <div class="session-create">
        <input v-model="newSessionTitle" placeholder="新会话标题..." class="input" @keyup.enter="handleCreateSession" />
        <button class="btn btn-primary" @click="handleCreateSession">创建</button>
      </div>
      <div class="session-list" v-if="ai.sessions.length > 0">
        <div v-for="s in ai.sessions" :key="s.id" class="session-item" :class="{ active: s.id === ai.currentSessionId }" @click="ai.currentSessionId = s.id">
          <span class="session-status-dot" :style="{ background: statusColors[s.status] }"></span>
          <span class="session-name">{{ s.title }}</span>
          <span class="session-meta">{{ s.workers.length }} workers</span>
        </div>
      </div>
      <div v-else class="empty-hint">暂无会话</div>
    </div>

    <!-- 任务规划 -->
    <div class="section" v-if="ai.currentSessionId">
      <div class="section-title font-gold">任务规划 (LLM Planner)</div>
      <textarea v-model="taskRequest" placeholder="描述你要完成的开发任务..." class="textarea" rows="3"></textarea>
      <button class="btn btn-primary" @click="handlePlanTask" :disabled="ai.isLoading">
        {{ ai.isLoading ? '拆解规划中...' : '规划任务' }}
      </button>
      <div class="task-tree" v-if="ai.taskTree.length > 0">
        <div v-for="task in ai.taskTree" :key="task.id" class="task-node" :class="{ selected: task.id === selectedTaskId }" @click="selectedTaskId = task.id">
          <div class="task-header">
            <span class="task-status-dot" :style="{ background: statusColors[task.status] }"></span>
            <span class="task-title">{{ task.title }}</span>
            <span class="task-priority">P{{ task.priority }}</span>
          </div>
          <div class="task-desc">{{ task.description }}</div>
          <div class="task-actions" v-if="task.status === 'pending'">
            <button class="btn btn-sm btn-primary" @click.stop="handleSpawnWorker(task)">启动 Worker</button>
          </div>
        </div>
      </div>
    </div>

    <!-- Worker 状态控制 -->
    <div class="section">
      <div class="section-title">Worker 调度池 <span class="count">{{ ai.workers.length }}</span></div>
      <div class="worker-list" v-if="ai.workers.length > 0">
        <div v-for="w in ai.workers" :key="w.id" class="worker-item">
          <div class="worker-header">
            <span class="worker-status-dot" :style="{ background: statusColors[w.status] }"></span>
            <span class="worker-id">{{ w.id.slice(0, 8) }}</span>
            <span class="worker-status font-gray">{{ statusLabel(w.status) }}</span>
            <button class="btn btn-sm btn-danger" @click="handleKillWorker(w.id)" v-if="!['killed','completed','failed'].includes(w.status)">终止</button>
          </div>
          <div class="worker-thought" v-if="w.currentThought">💭 {{ w.currentThought }}</div>
          
          <!-- COW 隔离路径展示 -->
          <div class="worker-sandbox" v-if="w.cowPath">
            <span class="sandbox-label">📦 COW 隔离沙箱:</span>
            <code class="sandbox-path" :title="w.cowPath">{{ w.cowPath }}</code>
          </div>

          <div class="worker-meta">
            <span>🔧 {{ w.toolCallCount }} 工具</span>
            <span v-if="w.consecutiveErrors > 0" class="error-count">❌ {{ w.consecutiveErrors }} 连错</span>
            <span>📊 {{ w.tokenCount }} tokens</span>
            <span class="cost-tracker">💰 ¥{{ estimateCost(w.tokenCount) }} 元</span>
          </div>
        </div>
      </div>
      <div v-else class="empty-hint">暂无 Worker 运行</div>
    </div>

    <!-- 知识经验草稿箱 (RAG 审阅) -->
    <div class="section">
      <div class="section-title">
        💡 知识经验草稿箱
        <span class="draft-badge" v-if="ai.drafts.length > 0">{{ ai.drafts.length }}</span>
      </div>
      <div class="draft-list" v-if="ai.drafts.length > 0">
        <div v-for="d in ai.drafts" :key="d.id" class="draft-item">
          <div class="draft-header">
            <span class="draft-title">{{ d.title }}</span>
            <span class="draft-conf" title="置信度">⭐ {{ d.confidence.toFixed(2) }}</span>
          </div>
          <div class="draft-content">{{ d.content }}</div>
          <div class="draft-actions">
            <button class="btn btn-sm btn-primary" @click="handlePromote(d.id)">✓ 推广为正式经验</button>
          </div>
        </div>
      </div>
      <div v-else class="empty-hint">暂无待审阅的知识草稿</div>
    </div>

    <!-- “反重力”毛玻璃聚合安全审批弹窗 -->
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
              <span class="approval-worker font-gray">Worker: {{ req.workerId.slice(0, 8) }}</span>
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
          <button class="btn btn-danger" @click="handleDenyAll">✗ 一键全部拒绝 (Reject All)</button>
          <button class="btn btn-success" @click="handleApproveAll">✓ 一键全部批准</button>
        </div>
      </div>
    </div>

    <div class="error-banner" v-if="ai.error">{{ ai.error }}</div>
  </div>
</template>

<style scoped lang="scss">
.ai-panel { display: flex; flex-direction: column; height: 100%; overflow-y: auto; padding: 16px; gap: 20px; background: var(--jc-bg-app); color: var(--jc-text-primary); }
.ai-header { display: flex; align-items: center; justify-content: space-between; padding-bottom: 10px; border-bottom: 1px solid var(--jc-border-default); }
.ai-title { font-size: 15px; font-weight: 700; margin: 0; color: var(--jc-color-accent); }
.ai-stats { display: flex; gap: 8px; }
.stat-badge { padding: 3px 10px; border-radius: 12px; font-size: 11px; background: var(--jc-bg-elevated); font-weight: 600; &.warn { background: rgba(210,153,34,0.2); color: #d29922; border: 1px solid rgba(210,153,34,0.4); } }

/* 工作区卡片 */
.workspace-card {
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--jc-border-default);
  border-radius: 8px;
  padding: 12px 14px;
  font-size: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  .workspace-meta { display: flex; gap: 8px; align-items: center; }
  .workspace-label { font-weight: 600; color: var(--jc-text-secondary); flex-shrink: 0; }
  .workspace-path { font-family: monospace; color: #58a6ff; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex: 1; }
  
  .workspace-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    
    .workspace-manual {
      display: flex;
      align-items: center;
      gap: 6px;
      flex: 1;
      justify-content: flex-end;
      .input-inline-sm {
        height: 24px;
        padding: 2px 8px;
        font-size: 11px;
        max-width: 180px;
      }
    }
  }
}

/* 配置折叠样式 */
.settings-section {
  .settings-body { display: flex; flex-direction: column; gap: 10px; padding-top: 10px; border-top: 1px dashed var(--jc-border-default); }
  .settings-row { display: flex; align-items: center; justify-content: space-between; font-size: 12px; label { color: var(--jc-text-secondary); } }
  .input-inline { width: 120px; text-align: right; }
  .settings-actions { display: flex; justify-content: flex-end; margin-top: 4px; }
}

.section { display: flex; flex-direction: column; gap: 10px; background: rgba(255,255,255,0.02); border: 1px solid var(--jc-border-default); border-radius: 8px; padding: 12px; }
.section-title { font-size: 13px; font-weight: 600; color: var(--jc-text-secondary); display: flex; align-items: center; justify-content: space-between; &.font-gold { color: #f0883e; } }
.draft-badge { padding: 1px 6px; border-radius: 10px; background: #58a6ff; color: #fff; font-size: 10px; font-weight: 700; }
.count { padding: 2px 7px; border-radius: 10px; font-size: 11px; background: var(--jc-bg-elevated); &.warn { background: rgba(210,153,34,0.2); color: #d29922; } }
.input, .textarea { padding: 8px 12px; border: 1px solid var(--jc-border-default); border-radius: 6px; background: var(--jc-bg-input); color: var(--jc-text-primary); font-size: 13px; outline: none; transition: border 0.2s; &:focus { border-color: var(--jc-color-accent); } }
.textarea { width: 100%; resize: vertical; font-family: inherit; }
.session-create { display: flex; gap: 8px; .input { flex: 1; } }
.session-list { display: flex; flex-direction: column; gap: 6px; }
.session-item { display: flex; align-items: center; gap: 10px; padding: 8px 12px; border-radius: 6px; cursor: pointer; font-size: 13px; border: 1px solid transparent; transition: all 0.2s; &:hover { background: var(--jc-bg-elevated); } &.active { background: var(--jc-bg-active); border-color: rgba(240,136,62,0.3); } }
.session-status-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; }
.session-name { flex: 1; font-weight: 500; }
.session-meta { font-size: 11px; color: var(--jc-text-secondary); }
.task-tree { display: flex; flex-direction: column; gap: 8px; margin-top: 8px; }
.task-node { padding: 10px 12px; border: 1px solid var(--jc-border-default); border-radius: 6px; cursor: pointer; background: rgba(255,255,255,0.01); transition: all 0.2s; &:hover { border-color: var(--jc-color-accent); } &.selected { border-color: var(--jc-color-accent); background: var(--jc-bg-active); } }
.task-header { display: flex; align-items: center; gap: 8px; }
.task-status-dot { width: 8px; height: 8px; border-radius: 50%; }
.task-title { flex: 1; font-size: 13px; font-weight: 600; }
.task-priority { font-size: 11px; color: var(--jc-text-secondary); font-weight: 600; }
.task-desc { font-size: 12px; color: var(--jc-text-secondary); margin-top: 5px; line-height: 1.4; }
.task-actions { margin-top: 8px; }
.worker-list, .draft-list { display: flex; flex-direction: column; gap: 8px; }
.worker-item, .draft-item { padding: 10px 12px; border: 1px solid var(--jc-border-default); border-radius: 6px; background: rgba(255,255,255,0.01); }
.worker-header, .draft-header { display: flex; align-items: center; gap: 8px; }
.worker-status-dot { width: 8px; height: 8px; border-radius: 50%; }
.worker-id { font-family: monospace; font-size: 12px; font-weight: 600; color: #58a6ff; }
.worker-status { flex: 1; font-size: 12px; }
.worker-thought { font-size: 12px; color: var(--jc-text-secondary); margin-top: 6px; font-style: italic; background: rgba(255,255,255,0.02); padding: 4px 8px; border-radius: 4px; border-left: 2px solid #a371f7; }

/* Sandbox COW 展示 */
.worker-sandbox {
  margin-top: 6px;
  background: rgba(0, 0, 0, 0.2);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 4px;
  padding: 6px 10px;
  font-size: 11px;
  display: flex;
  gap: 6px;
  align-items: center;
  .sandbox-label { font-weight: 600; color: var(--jc-text-secondary); flex-shrink: 0; }
  .sandbox-path { font-family: monospace; color: #a371f7; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; flex: 1; }
}

.worker-meta { display: flex; gap: 14px; margin-top: 6px; font-size: 11px; color: var(--jc-text-secondary); .error-count { color: #f85149; font-weight: 700; } .cost-tracker { color: #3fb950; font-weight: 700; } }

/* 草稿箱样式 */
.draft-title { flex: 1; font-size: 13px; font-weight: 600; }
.draft-conf { font-size: 11px; font-weight: 700; color: #d29922; }
.draft-content { font-size: 12px; color: var(--jc-text-secondary); margin: 6px 0; line-height: 1.4; white-space: pre-wrap; background: rgba(0,0,0,0.15); padding: 6px; border-radius: 4px; }
.draft-actions { display: flex; justify-content: flex-end; }

/* “反重力”毛玻璃聚合审批弹窗样式 */
.overlay-backdrop { position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0, 0, 0, 0.55); backdrop-filter: blur(10px); z-index: 9999; display: flex; align-items: center; justify-content: center; padding: 20px; }
.overlay-card { width: 100%; max-width: 580px; max-height: 85vh; background: var(--jc-bg-app); border: 1px solid var(--jc-border-default); border-radius: 12px; box-shadow: 0 12px 40px rgba(0,0,0,0.4); display: flex; flex-direction: column; overflow: hidden; }
.overlay-header { display: flex; align-items: center; justify-content: space-between; padding: 14px 18px; border-bottom: 1px solid var(--jc-border-default); background: rgba(255,255,255,0.01); }
.overlay-title { font-size: 15px; font-weight: 700; margin: 0; color: #f0883e; }
.overlay-badge { font-size: 11px; font-weight: 600; padding: 2px 8px; border-radius: 10px; background: rgba(248,81,73,0.15); color: #f85149; }
.overlay-body { flex: 1; overflow-y: auto; padding: 18px; display: flex; flex-direction: column; gap: 14px; }
.approval-card-item { border: 1px solid var(--jc-border-default); border-radius: 8px; padding: 12px; background: rgba(255,255,255,0.02); }
.approval-card-meta { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.risk-badge { padding: 2px 7px; border-radius: 8px; font-size: 9px; font-weight: 700; color: #fff; text-transform: uppercase; }
.approval-tool { flex: 1; font-weight: 600; font-size: 13px; }
.approval-worker { font-family: monospace; font-size: 11px; }
.approval-card-reason { font-size: 12px; color: var(--jc-text-secondary); line-height: 1.4; }
.approval-card-args { font-size: 11px; margin-top: 6px; background: rgba(0,0,0,0.1); padding: 4px 8px; border-radius: 4px; word-break: break-all; }
.approval-card-diff { margin-top: 8px; padding: 8px; background: #0c1117; border: 1px solid var(--jc-border-default); border-radius: 6px; font-size: 11px; font-family: monospace; overflow-x: auto; max-height: 180px; color: #c9d1d9; white-space: pre-wrap; line-height: 1.4; }
.approval-card-actions { display: flex; gap: 8px; margin-top: 10px; justify-content: flex-end; }
.overlay-footer { padding: 14px 18px; border-top: 1px solid var(--jc-border-default); background: rgba(255,255,255,0.01); display: flex; justify-content: space-between; gap: 12px; }

.btn { padding: 6px 14px; border: 1px solid var(--jc-border-default); border-radius: 6px; background: var(--jc-bg-elevated); color: var(--jc-text-primary); font-size: 13px; cursor: pointer; font-weight: 500; transition: all 0.2s; &:hover { background: var(--jc-bg-hover); } &:disabled { opacity: 0.5; cursor: not-allowed; } &.btn-primary { background: var(--jc-color-accent); color: #fff; border-color: var(--jc-color-accent); &:hover { opacity: 0.9; } } &.btn-success { background: #238636; color: #fff; border-color: #2ea043; &:hover { background: #2ea043; } } &.btn-danger { background: #da3633; color: #fff; border-color: #f85149; &:hover { background: #f85149; } } &.btn-sm { padding: 3px 10px; font-size: 12px; border-radius: 4px; } }
.empty-hint { font-size: 12px; color: var(--jc-text-secondary); text-align: center; padding: 16px; border: 1px dashed var(--jc-border-default); border-radius: 6px; }
.error-banner { padding: 10px 14px; background: rgba(248,81,73,0.08); border: 1px solid #f85149; border-radius: 6px; font-size: 12px; color: #f85149; margin-top: 8px; }

/* 实用颜色辅助 */
.font-gold { color: #f0883e !important; }
.font-gray { color: var(--jc-text-secondary) !important; }
</style>