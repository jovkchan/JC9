// AI Agent 类型定义 - 与后端 types.rs 对应

export enum SessionStatus {
  Active = 'active',
  Paused = 'paused',
  Completed = 'completed',
  Failed = 'failed',
}

export enum TaskStatus {
  Pending = 'pending',
  InProgress = 'inProgress',
  Completed = 'completed',
  Failed = 'failed',
  Blocked = 'blocked',
}

export enum WorkerStatus {
  Thinking = 'thinking',
  Acting = 'acting',
  Observing = 'observing',
  WaitingApproval = 'waitingApproval',
  Completed = 'completed',
  Failed = 'failed',
  Killed = 'killed',
}

export enum RiskLevel {
  Low = 'low',
  Medium = 'medium',
  High = 'high',
  Critical = 'critical',
}

export enum ApprovalStatus {
  Pending = 'pending',
  Approved = 'approved',
  Denied = 'denied',
  Expired = 'expired',
}

export enum KbEntryType {
  CodePattern = 'codePattern',
  ApiReference = 'apiReference',
  ErrorSolution = 'errorSolution',
  BestPractice = 'bestPractice',
  ProjectContext = 'projectContext',
  UserPreference = 'userPreference',
}

export interface Session {
  id: string
  title: string
  status: SessionStatus
  createdAt: string
  updatedAt: string
  taskTree: TaskNode[]
  workers: string[]
  blackboardSnapshot: unknown | null
  totalTokens: number
  totalCost: number
}

export interface TaskNode {
  id: string
  parentId: string | null
  sessionId: string
  title: string
  description: string
  status: TaskStatus
  priority: number
  assignedWorker: string | null
  subTasks: string[]
  createdAt: string
  updatedAt: string
  result: string | null
}

export interface ToolCallRecord {
  id: string
  toolName: string
  arguments: Record<string, unknown>
  result: unknown | null
  status: 'pending' | 'approved' | 'denied' | 'executing' | 'success' | 'failed'
  timestamp: string
}

export interface ReActStep {
  iteration: number
  thought: string
  action: ToolCallRecord | null
  observation: string | null
  timestamp: string
}

export interface WorkerState {
  id: string
  sessionId: string
  taskId: string
  status: WorkerStatus
  currentThought: string | null
  toolCallCount: number
  consecutiveErrors: number
  lastErrorHash: string | null
  createdAt: string
  lastActive: string
  tokenCount: number
  history: ReActStep[]
  terminationReason?: string | null
}

export interface ApprovalRequest {
  id: string
  workerId: string
  sessionId: string
  toolName: string
  arguments: Record<string, unknown>
  riskLevel: RiskLevel
  reason: string
  diffPreview: string | null
  status: ApprovalStatus
  createdAt: string
  resolvedAt: string | null
}

export interface KbEntry {
  id: string
  entryType: KbEntryType
  title: string
  content: string
  tags: string[]
  source: string
  confidence: number
  isDraft: boolean
  createdAt: string
  updatedAt: string
}

export interface McpServer {
  id: string
  name: string
  transport: 'sse' | 'stdio'
  url?: string | null
  command?: string | null
  args?: string[] | null
  status: 'connected' | 'disconnected' | 'error' | 'connecting'
  tools: { name: string; description: string; inputSchema: unknown }[]
  errorMessage?: string | null
}

export interface ToolCall {
  toolName: string
  arguments: Record<string, unknown>
}