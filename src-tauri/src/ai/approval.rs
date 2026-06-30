use std::sync::Arc;
use tokio::sync::{RwLock, Notify};
use std::collections::HashMap;
use chrono::Utc;
use tauri::Emitter;
use super::types::*;

/// 审批队列 - 高风险工具调用的人工授权拦截
pub struct ApprovalQueue {
    pending: Arc<RwLock<HashMap<String, ApprovalRequest>>>,
    resolved: Arc<RwLock<Vec<ApprovalRequest>>>,
    auto_approve_low_risk: Arc<RwLock<bool>>,
    notifiers: Arc<RwLock<HashMap<String, Arc<Notify>>>>,
    app_handle: Option<tauri::AppHandle>,
}

impl ApprovalQueue {
    pub fn new(app_handle: Option<tauri::AppHandle>) -> Self {
        Self {
            pending: Arc::new(RwLock::new(HashMap::new())),
            resolved: Arc::new(RwLock::new(Vec::new())),
            auto_approve_low_risk: Arc::new(RwLock::new(false)),
            notifiers: Arc::new(RwLock::new(HashMap::new())),
            app_handle,
        }
    }

    pub async fn needs_approval(&self, tool_name: &str) -> bool {
        ["write_file", "patch_file", "run_command"].contains(&tool_name)
    }

    pub async fn request_approval(&self, mut req: ApprovalRequest) -> bool {
        if req.risk_level == RiskLevel::Low && *self.auto_approve_low_risk.read().await {
            req.status = ApprovalStatus::Approved;
            req.resolved_at = Some(Utc::now());
            self.resolved.write().await.push(req.clone());
            if let Some(ref handle) = self.app_handle {
                let _ = handle.emit("ai:approval-request", req);
            }
            return true;
        }
        let req_id = req.id.clone();
        let notify = Arc::new(Notify::new());
        self.notifiers.write().await.insert(req_id.clone(), notify.clone());
        self.pending.write().await.insert(req_id.clone(), req.clone());

        if let Some(ref handle) = self.app_handle {
            let _ = handle.emit("ai:approval-request", req);
        }

        let timeout = tokio::time::Duration::from_secs(300);
        match tokio::time::timeout(timeout, notify.notified()).await {
            Ok(()) => {
                self.resolved.read().await.iter().find(|r| r.id == req_id)
                    .map(|r| r.status == ApprovalStatus::Approved).unwrap_or(false)
            }
            Err(_) => {
                let mut pending = self.pending.write().await;
                if let Some(mut req) = pending.remove(&req_id) {
                    req.status = ApprovalStatus::Expired;
                    req.resolved_at = Some(Utc::now());
                    self.resolved.write().await.push(req.clone());
                    if let Some(ref handle) = self.app_handle {
                        let _ = handle.emit("ai:approval-request", req);
                    }
                }
                self.notifiers.write().await.remove(&req_id);
                false
            }
        }
    }

    pub async fn approve(&self, request_id: &str) -> bool {
        let mut pending = self.pending.write().await;
        if let Some(mut req) = pending.remove(request_id) {
            req.status = ApprovalStatus::Approved;
            req.resolved_at = Some(Utc::now());
            self.resolved.write().await.push(req.clone());
            drop(pending);
            if let Some(ref handle) = self.app_handle {
                let _ = handle.emit("ai:approval-request", req);
            }
            if let Some(notify) = self.notifiers.read().await.get(request_id) { notify.notify_one(); }
            self.notifiers.write().await.remove(request_id);
            true
        } else { false }
    }

    pub async fn deny(&self, request_id: &str) -> bool {
        let mut pending = self.pending.write().await;
        if let Some(mut req) = pending.remove(request_id) {
            req.status = ApprovalStatus::Denied;
            req.resolved_at = Some(Utc::now());
            self.resolved.write().await.push(req.clone());
            drop(pending);
            if let Some(ref handle) = self.app_handle {
                let _ = handle.emit("ai:approval-request", req);
            }
            if let Some(notify) = self.notifiers.read().await.get(request_id) { notify.notify_one(); }
            self.notifiers.write().await.remove(request_id);
            true
        } else { false }
    }

    /// 拒绝所有待审批请求
    pub async fn deny_all(&self) {
        let ids: Vec<String> = {
            let pending = self.pending.read().await;
            pending.keys().cloned().collect()
        };
        for id in ids {
            self.deny(&id).await;
        }
    }

    pub async fn get_pending(&self) -> Vec<ApprovalRequest> { self.pending.read().await.values().cloned().collect() }
    pub async fn get_resolved(&self) -> Vec<ApprovalRequest> { self.resolved.read().await.clone() }
    pub async fn set_auto_approve_low_risk(&self, enabled: bool) { *self.auto_approve_low_risk.write().await = enabled; }
}