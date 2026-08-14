// ── 自动化积木编辑器（Block Automation Editor）—— 前端与 Rust 共享契约 ──
// 唯一事实源：契约冻结后不改字段名（见 docs/plans/2026-08-03-block-automation-editor.md §7.3）

export type BlockCategory =
  | 'entry' | 'terminal' | 'platform' | 'env'
  | 'logic' | 'variable' | 'ai' | 'credential' | 'end' | 'scm'

export type PortDataType = 'flow' | 'data-string' | 'data-number' | 'data-bool' | 'credential'

export interface Port {
  id: string
  direction: 'in' | 'out'
  color: string          // 由 dataType 决定
  dataType: PortDataType
  label?: string
  /** 可连接多条线（分支端口）→ 渲染为琥珀色高亮 */
  multi?: boolean
}

export interface BlockNode {
  id: string
  type: string           // 注册表中的积木 key
  category: BlockCategory
  x: number
  y: number
  config: Record<string, unknown>  // 由积木 schema 校验
  /** 固定位置：锁定后不可拖拽（右键「固定」切换） */
  locked?: boolean
}

/** 积木「登录」配置：绑定凭据后执行前先鉴权（见方案 §6） */
export interface LoginConfig {
  credentialId: string
  credentialName: string
  credentialKind?: CredentialKind
  /** 登录目标平台/主机（docker/gitlab/jenkins/harbor/k8s/ssh） */
  platform?: string
}

// ── 执行日志（结构化，每个积木执行都记录；引擎写入 automation_logs.json，最新在前）──
export interface RunStepLog {
  blockId: string
  blockType: string
  name: string
  index: number
  status: 'ok' | 'fail'
  startedAt: number
  endedAt: number
  durationMs: number
  exitCode: number | null
  stdoutTail: string
  /** 实际执行内容（已插值）：命令 / git / curl / 网址 / 程序等 */
  detail: string
  cwd: string
  /** 鉴权信息（凭据名，不含明文） */
  auth: string
  iteration?: number
  branch?: number
}

export interface RunLog {
  id: string
  automationId: string
  automationName: string
  entry: string
  status: 'done' | 'failed' | 'stopped'
  startedAt: number
  endedAt: number
  durationMs: number
  error: string | null
  steps: RunStepLog[]
}

export interface Edge {
  id: string
  fromBlock: string
  fromPort: string
  toBlock: string
  toPort: string
}

export interface Automation {
  id: string
  name: string
  description: string
  version: 2            // 数据模型版本
  nodes: BlockNode[]
  edges: Edge[]
  variables: Record<string, { type: 'string' | 'number' | 'boolean'; value: unknown }>
  createdAt: string
  updatedAt: string
}

// ── 凭据 ──
export type CredentialKind =
  | 'basic'        // 用户名 + 密码
  | 'pat'          // Personal Access Token
  | 'token'        // API Token
  | 'ssh-key'      // 私钥
  | 'kubeconfig'   // K8S 配置

export interface Credential {
  id: string
  name: string
  kind: CredentialKind
  /** 引用哪个平台（docker/gitlab/jenkins/harbor/k8s） */
  platform: string
  /** 敏感字段加密存储；frontend 只见掩码 */
  fields: { username?: string; password?: string; token?: string; url?: string }
  createdAt: string
}

/** 列表视图：仅掩码（前端不持有明文） */
export interface CredentialMeta {
  id: string
  name: string
  kind: CredentialKind
  platform: string
  masked: Record<string, string>   // 如 { token: 'glpat-****1234' }
  createdAt: string
}

/** 保存入参：一次性明文，保存后不回流 */
export interface CredentialInput {
  id?: string
  name: string
  kind: CredentialKind
  platform: string
  fields: { username?: string; password?: string; token?: string; url?: string }
}

// ── 积木注册表（Schema 驱动）──
export type FieldType =
  | 'text' | 'textarea' | 'number' | 'select' | 'switch'
  | 'shell' | 'credential' | 'env' | 'var'

export interface FieldDef {
  key: string
  label: string
  type: FieldType
  required?: boolean
  placeholder?: string
  default?: unknown
  options?: { label: string; value: unknown }[]   // select
  /** 是否支持 {{var}} / {{last.*}} 插值提示 */
  interpolatable?: boolean
  help?: string
  /** 路径/文件字段的选择器：dir = 目录选择；file = 文件选择 */
  picker?: 'dir' | 'file'
}

export interface PortCompatibility {
  /** 出端口类型 */
  from: PortDataType
  /** 允许接入的入端口类型白名单；缺省 = 同类型 */
  to?: PortDataType[]
  /** 出端口可接任意类型入端口（如凭据端口接各平台块） */
  acceptsAny?: boolean
}

export interface BlockDef {
  type: string
  category: BlockCategory
  label: string
  color: string             // 块主题色
  inputs: Port[]            // 默认 in 端口（端口 id 必须语义化稳定：'in'/'out'/'out-true'...）
  outputs: Port[]           // 默认 out 端口
  /** 参数 schema：驱动 InspectorPanel 表单 + AI 生成 */
  fields: FieldDef[]
  /** 端口兼容规则（数据化，前端/Rust/AI 共用） */
  compatRules: PortCompatibility[]
}

// ── 上下文（块间传值）──
export type ShellKind = 'powershell' | 'cmd' | 'bash' | 'sh' | 'python' | 'node'

export interface LastResult {
  blockId: string
  exitCode: number
  stdout: string
  stderr: string
}

/** 块对链路上文做的覆盖（可选，默认继承） */
export interface BlockContextOverride {
  cwd?: string
  shell?: ShellKind
  env?: Record<string, string>
  /** 输出捕获：把 stdout 写入变量 */
  capture?: { varName: string; pattern?: string; group?: number }
}

/** 运行时链路上下文（Rust 端维护，前端只读快照） */
export interface ChainContext {
  cwd: string
  shell: ShellKind
  env: Record<string, string>
  sessions: Record<string, unknown>
  last: LastResult | null
  vars: Record<string, unknown>
}

// ── 事件（mock 与真实引擎同构）──
export type AutomationEventType =
  | 'step_start' | 'step_done' | 'step_fail' | 'var_change'
  | 'cred_used' | 'done' | 'stopped' | 'error'

export interface AutomationEvent {
  type: AutomationEventType
  runId: string
  blockId?: string
  step?: number
  total?: number
  name?: string
  exitCode?: number
  stdoutTail?: string            // 输出尾部摘要（终端块）
  vars?: Record<string, unknown> // 变量快照（step 粒度）
  ts: number
}

// ── transport 统一接口（mock 与 invoke 都实现；前端业务只依赖它）──
export interface RunHandle {
  stop(): Promise<void>
}

export interface AutomationTransport {
  // 自动化 CRUD
  list(): Promise<Automation[]>
  save(a: Automation): Promise<void>
  delete(id: string): Promise<void>
  // 凭据 CRUD（F3）—— 前端不持有明文
  credentialList(): Promise<CredentialMeta[]>
  credentialSave(c: CredentialInput): Promise<void>
  credentialDelete(id: string): Promise<void>
  // 执行
  run(id: string): Promise<RunHandle>
  probeVersion(platform: string): Promise<string>
  // 事件
  onEvent(cb: (e: AutomationEvent) => void): () => void
}
