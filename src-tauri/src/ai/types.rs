use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ══════════════════════════════════════════════════════════════
// AI Agent 核心类型定义
// ══════════════════════════════════════════════════════════════

/// AI 会话
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSession {
    pub id: String,
    pub title: String,
    pub status: SessionStatus,
    pub project_id: Option<String>,
    pub task_description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub token_count: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// AI 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiMessage {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    pub thought: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub observation: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub token_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 工具调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallRecord {
    pub id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub status: ToolCallStatus,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolCallStatus {
    Pending,
    Approved,
    Denied,
    Executing,
    Success,
    Failed,
}

/// 任务树节点
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub session_id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: i32,
    pub assigned_worker: Option<String>,
    pub sub_tasks: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Failed,
    Cancelled,
}

/// Worker 代理状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerState {
    pub id: String,
    pub session_id: String,
    pub task_id: String,
    pub status: WorkerStatus,
    pub current_thought: Option<String>,
    pub tool_call_count: u32,
    pub consecutive_errors: u32,
    pub last_error_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub token_count: u64,
    pub cow_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkerStatus {
    Idle,
    Thinking,
    CallingTool,
    WaitingApproval,
    Observing,
    Reflecting,
    Completed,
    Failed,
    Killed,
}

/// 黑板条目 - 遵循文档规约的严格 JSON Schema
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlackboardEntry {
    pub id: String,
    pub entry_type: BlackboardEntryType,
    pub key: String,
    pub value: String,
    pub source_worker: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BlackboardEntryType {
    GlobalConfigPath,
    DependencyResolved,
    EnvVariable,
    IdentifiedBug,
    TaskProgress,
    SharedContext,
    ErrorPattern,
}

/// 审批请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalRequest {
    pub id: String,
    pub worker_id: String,
    pub session_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub risk_level: RiskLevel,
    pub reason: String,
    pub diff_preview: Option<String>,
    pub status: ApprovalStatus,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

/// 知识库条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KbEntry {
    pub id: String,
    pub title: String,
    pub content: String,
    pub entry_type: KbEntryType,
    pub tags: Vec<String>,
    pub source_session: Option<String>,
    pub confidence: f64,
    pub is_draft: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum KbEntryType {
    PitfallNote,
    Solution,
    Pattern,
    ApiReference,
    ConfigNote,
    Takeaway,
}

/// ReAct 循环状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReActState {
    pub worker_id: String,
    pub iteration: u32,
    pub history: Vec<ReActStep>,
    pub is_terminated: bool,
    pub termination_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReActStep {
    pub iteration: u32,
    pub thought: String,
    pub action: Option<ToolCallRecord>,
    pub observation: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// 熔断器状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopBreakerState {
    pub worker_id: String,
    pub tool_call_count: u32,
    pub consecutive_errors: u32,
    pub error_hashes: Vec<String>,
    pub warning_injected: bool,
    pub is_tripped: bool,
    pub trip_reason: Option<String>,
}

/// 工作区隔离状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceBranch {
    pub id: String,
    pub worker_id: String,
    pub session_id: String,
    pub original_path: String,
    pub branch_path: String,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceStatus {
    Active,
    Merged,
    Discarded,
    Conflict,
}

/// 合并冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeConflict {
    pub file_path: String,
    pub base_content: String,
    pub ours_content: String,
    pub theirs_content: String,
    pub conflict_type: ConflictType,
    pub resolution: Option<ConflictResolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    ContentConflict,
    StructuralConflict,
    SemanticConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictResolution {
    pub resolved_content: String,
    pub resolved_by: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostConfig {
    pub input_cached_cost_per_m: f64,
    pub input_uncached_cost_per_m: f64,
    pub output_cost_per_m: f64,
    pub cost_limit: f64,
}

/// Token 成本看板
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenCostTracker {
    pub session_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_cost_usd: f64,
    pub cost_limit_usd: f64,
    pub is_circuit_broken: bool,
    pub total_cost_cny: f64,
}

/// 宿主环境信息（脱敏后）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEnvironment {
    pub os: String,
    pub os_version: String,
    pub shell: String,
    pub arch: String,
    pub env_vars: Vec<EnvVarEntry>,
    pub cli_versions: Vec<CliVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarEntry {
    pub key: String,
    pub value: String,
    pub is_sensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliVersion {
    pub name: String,
    pub version: String,
}