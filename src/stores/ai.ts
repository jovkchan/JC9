import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type {
  Session,
  TaskNode,
  WorkerState,
  ApprovalRequest,
  KbEntry,
  McpServer,
} from '@/types/ai'

export const useAiStore = defineStore('ai', () => {
  // ── State ──
  const sessions = ref<Session[]>([])
  const currentSessionId = ref<string | null>(null)
  const workers = ref<WorkerState[]>([])
  const pendingApprovals = ref<ApprovalRequest[]>([])
  const knowledgeEntries = ref<KbEntry[]>([])
  const drafts = ref<KbEntry[]>([])
  const mcpServers = ref<McpServer[]>([])
  const workspaceRoot = ref<string>('')
  const costConfig = ref({
    inputCachedCostPerM: 0.025,
    inputUncachedCostPerM: 3.0,
    outputCostPerM: 6.0,
    costLimit: 5.0
  })
  const isLoading = ref(false)
  const error = ref<string | null>(null)
  const taskTree = ref<TaskNode[]>([])

  // ── Getters ──
  const currentSession = computed(() =>
    sessions.value.find((s) => s.id === currentSessionId.value) ?? null,
  )
  const activeWorkers = computed(() =>
    workers.value.filter(
      (w) =>
        w.status !== 'completed' &&
        w.status !== 'failed' &&
        w.status !== 'killed',
    ),
  )
  const pendingApprovalsCount = computed(() => pendingApprovals.value.length)

  // ── Actions ──
  async function loadSessions() {
    isLoading.value = true
    error.value = null
    try {
      sessions.value = await invoke<Session[]>('ai_list_sessions')
    } catch (e) {
      error.value = String(e)
    } finally {
      isLoading.value = false
    }
  }

  async function createSession(title: string) {
    isLoading.value = true
    error.value = null
    try {
      const id = await invoke<string>('ai_create_session', { title })
      currentSessionId.value = id
      await loadSessions()
      return id
    } catch (e) {
      error.value = String(e)
      return null
    } finally {
      isLoading.value = false
    }
  }

  async function planTask(sessionId: string, request: string) {
    isLoading.value = true
    error.value = null
    try {
      taskTree.value = await invoke<TaskNode[]>('ai_plan_task', {
        sessionId,
        request,
      })
      return taskTree.value
    } catch (e) {
      error.value = String(e)
      return []
    } finally {
      isLoading.value = false
    }
  }

  async function spawnWorker(
    sessionId: string,
    task: TaskNode,
    systemPrompt: string,
  ) {
    error.value = null
    try {
      const workerId = await invoke<string>('ai_spawn_worker', {
        sessionId,
        task,
        systemPrompt,
      })
      await loadWorkers()
      return workerId
    } catch (e) {
      error.value = String(e)
      return null
    }
  }

  async function loadWorkers() {
    try {
      workers.value = await invoke<WorkerState[]>('ai_list_workers')
    } catch (e) {
      error.value = String(e)
    }
  }

  async function killWorker(workerId: string) {
    try {
      await invoke('ai_kill_worker', { workerId })
      await loadWorkers()
    } catch (e) {
      error.value = String(e)
    }
  }

  async function loadPendingApprovals() {
    try {
      pendingApprovals.value = await invoke<ApprovalRequest[]>(
        'ai_get_pending_approvals',
      )
    } catch (e) {
      error.value = String(e)
    }
  }

  async function approveRequest(requestId: string) {
    try {
      await invoke('ai_approve_request', { requestId })
      await loadPendingApprovals()
    } catch (e) {
      error.value = String(e)
    }
  }

  async function denyRequest(requestId: string) {
    try {
      await invoke('ai_deny_request', { requestId })
      await loadPendingApprovals()
    } catch (e) {
      error.value = String(e)
    }
  }

  async function searchKnowledge(query: string, limit?: number) {
    try {
      knowledgeEntries.value = await invoke<KbEntry[]>('ai_search_knowledge', {
        query,
        limit: limit ?? 10,
      })
    } catch (e) {
      error.value = String(e)
    }
  }

  async function addKnowledge(entry: KbEntry) {
    try {
      await invoke('ai_add_knowledge', { entry })
    } catch (e) {
      error.value = String(e)
    }
  }

  async function connectMcpServer(name: string, url: string) {
    try {
      await invoke('ai_connect_mcp_server', { name, url })
      await loadMcpServers()
    } catch (e) {
      error.value = String(e)
    }
  }

  async function loadMcpServers() {
    try {
      mcpServers.value = await invoke<McpServer[]>('ai_list_mcp_servers')
    } catch (e) {
      error.value = String(e)
    }
  }

  async function loadDrafts() {
    try {
      drafts.value = await invoke<KbEntry[]>('ai_list_drafts')
    } catch (e) {
      error.value = String(e)
    }
  }

  async function promoteKnowledge(entryId: string) {
    try {
      await invoke('ai_promote_knowledge', { entryId })
      await loadDrafts()
    } catch (e) {
      error.value = String(e)
    }
  }

  async function loadWorkspaceRoot() {
    try {
      workspaceRoot.value = await invoke<string>('ai_get_workspace_root')
    } catch (e) {
      error.value = String(e)
    }
  }

  async function changeWorkspaceDialog() {
    try {
      const selected = await invoke<string | null>('ai_select_workspace_dialog')
      if (selected) {
        workspaceRoot.value = selected
      }
    } catch (e) {
      error.value = String(e)
    }
  }

  async function changeWorkspaceManual(newPath: string) {
    try {
      await invoke('ai_update_workspace_root', { newPath })
      workspaceRoot.value = newPath
    } catch (e) {
      error.value = String(e)
    }
  }

  async function updateCostConfig(config: typeof costConfig.value) {
    try {
      await invoke('ai_update_cost_config', { config })
      costConfig.value = config
    } catch (e) {
      error.value = String(e)
    }
  }

  // ── Event Listeners ──
  const unlisteners: Array<() => void> = []

  async function initListeners() {
    unlisteners.push(
      await listen<WorkerState>('ai:worker-update', (event) => {
        const idx = workers.value.findIndex((w) => w.id === event.payload.id)
        if (idx >= 0) {
          workers.value[idx] = event.payload
        } else {
          workers.value.push(event.payload)
        }
      }),
    )

    unlisteners.push(
      await listen<ApprovalRequest>('ai:approval-request', (event) => {
        const idx = pendingApprovals.value.findIndex(
          (a) => a.id === event.payload.id,
        )
        if (idx >= 0) {
          pendingApprovals.value[idx] = event.payload
        } else {
          pendingApprovals.value.push(event.payload)
        }
      }),
    )

    unlisteners.push(
      await listen<TaskNode>('ai:task-update', (event) => {
        const idx = taskTree.value.findIndex((t) => t.id === event.payload.id)
        if (idx >= 0) {
          taskTree.value[idx] = event.payload
        } else {
          taskTree.value.push(event.payload)
        }
      }),
    )
  }

  function destroyListeners() {
    unlisteners.forEach((fn) => fn())
    unlisteners.length = 0
  }

  return {
    sessions,
    currentSessionId,
    workers,
    pendingApprovals,
    knowledgeEntries,
    drafts,
    workspaceRoot,
    costConfig,
    mcpServers,
    isLoading,
    error,
    taskTree,
    currentSession,
    activeWorkers,
    pendingApprovalsCount,
    loadSessions,
    createSession,
    planTask,
    spawnWorker,
    loadWorkers,
    killWorker,
    loadPendingApprovals,
    approveRequest,
    denyRequest,
    searchKnowledge,
    addKnowledge,
    connectMcpServer,
    loadMcpServers,
    loadDrafts,
    promoteKnowledge,
    loadWorkspaceRoot,
    changeWorkspaceDialog,
    changeWorkspaceManual,
    updateCostConfig,
    initListeners,
    destroyListeners,
  }
})