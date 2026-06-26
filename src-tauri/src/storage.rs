use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub id: String,
    pub name: String,
    pub command: String,
    pub working_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub commands: Vec<Command>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    pub command: String,
    pub category: String,
    pub description: String,
}

fn get_storage_path() -> PathBuf {
    let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
    path.push("jc9-projects.json");
    path
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(PathBuf::from)
            .map(|p| p.join("jc9"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        dirs::data_dir().map(|p| p.join("jc9"))
    }
}

pub fn load_projects() -> Result<Vec<Project>, String> {
    let path = get_storage_path();
    if !path.exists() {
        // ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        // return empty
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content).map_err(|e| format!("解析数据失败: {}", e))
}

pub fn save_projects(projects: &[Project]) -> Result<(), String> {
    let path = get_storage_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(projects).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

fn get_shortcuts_path() -> PathBuf {
    let mut path = dirs_next().unwrap_or_else(|| PathBuf::from("."));
    path.push("jc9-shortcuts.json");
    path
}

/// Returns default built-in shortcuts + user-added ones
pub fn load_shortcuts() -> Vec<Shortcut> {
    let mut builtin = vec![
        // GO - 完整命令集
        s("go-bug","go bug","Go","启动 Go bug 报告"),
        s("go-build","go build -o bin/app.exe .","Go","编译包和依赖"),
        s("go-clean","go clean","Go","清除编译生成的对象文件和缓存"),
        s("go-doc","go doc","Go","显示包或符号的文档"),
        s("go-env","go env","Go","打印 Go 环境信息"),
        s("go-fix","go fix ./...","Go","更新包以使用新 API"),
        s("go-fmt","go fmt ./...","Go","格式化包源代码 (gofmt)"),
        s("go-generate","go generate ./...","Go","通过处理源代码生成 Go 文件"),
        s("go-get","go get","Go","添加依赖到当前模块并安装"),
        s("go-install","go install","Go","编译并安装包和依赖"),
        s("go-list","go list ./...","Go","列出包或模块"),
        s("go-mod-tidy","go mod tidy","Go","模块维护 - 整理依赖"),
        s("go-mod-verify","go mod verify","Go","模块维护 - 验证依赖"),
        s("go-mod-vendor","go mod vendor","Go","模块维护 - 复制依赖到 vendor"),
        s("go-work","go work","Go","工作区维护"),
        s("go-run","go run .","Go","编译并运行 Go 程序"),
        s("go-telemetry","go telemetry","Go","管理遥测数据和设置"),
        s("go-test","go test ./...","Go","测试包"),
        s("go-test-cover","go test -cover ./...","Go","测试包 (含覆盖率)"),
        s("go-tool","go tool","Go","运行指定的 Go 工具"),
        s("go-version","go version","Go","打印 Go 版本"),
        s("go-vet","go vet ./...","Go","报告包中可能的错误"),
        // NODE
        s("npm-install","npm install","Node","安装项目依赖"),
        s("npm-run-dev","npm run dev","Node","启动开发服务器"),
        s("npm-run-build","npm run build","Node","构建生产版本"),
        s("npm-audit","npm audit fix","Node","审计并修复依赖漏洞"),
        s("npm-init","npm init -y","Node","初始化 package.json"),
        s("yarn-install","yarn install","Node","yarn 安装依赖"),
        s("yarn-dev","yarn dev","Node","yarn 启动开发服务器"),
        s("npx-tsc","npx tsc --noEmit","Node","TypeScript 类型检查"),
        // GIT
        s("git-status","git status","Git","查看工作区状态"),
        s("git-pull","git pull","Git","拉取远程仓库更新"),
        s("git-push","git push","Git","推送本地提交到远程"),
        s("git-commit","git commit -m \"\"","Git","提交暂存区更改"),
        s("git-add-all","git add .","Git","暂存所有更改"),
        s("git-log","git log --oneline -10","Git","查看最近 10 条提交"),
        s("git-branch","git branch -a","Git","查看所有分支"),
        s("git-checkout","git checkout ","Git","切换分支"),
        s("git-stash","git stash","Git","暂存工作区更改"),
        s("git-diff","git diff","Git","查看未暂存的更改"),
        s("git-clone","git clone ","Git","克隆远程仓库"),
        s("git-merge","git merge ","Git","合并分支"),
        s("git-rebase","git rebase ","Git","变基操作"),
    ];
    let path = get_shortcuts_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(mut user) = serde_json::from_str::<Vec<Shortcut>>(&content) {
                builtin.extend(user);
            }
        }
    }
    builtin
}

pub fn save_shortcuts(shortcuts: &[Shortcut]) -> Result<(), String> {
    let path = get_shortcuts_path();
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    let content = serde_json::to_string_pretty(shortcuts).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

fn s(id: &str, cmd: &str, cat: &str, desc: &str) -> Shortcut {
    Shortcut { id: id.into(), name: cmd.into(), command: cmd.into(), category: cat.into(), description: desc.into() }
}
