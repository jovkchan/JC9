---
name: add-new-command
description: 添加新的 Tauri command 的完整流程
type: workflow
scope: dev
version: 1
---

# 添加新 Tauri Command 工作流

## 步骤

### Step 1: 在 `lib.rs` 中定义函数

```rust
#[tauri::command]
fn your_command(
    state: State<'_, Mutex<AppState>>,
    param1: String,
) -> Result<String, String> {
    let app_state = state.lock().map_err(|e| e.to_string())?;
    app_state.db.some_method(&param1)
}
```

如果是异步函数：

```rust
#[tauri::command]
async fn your_async_command(
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let ai_manager = {
        let app_state = state.lock().map_err(|e| e.to_string())?;
        app_state.ai_manager.clone()
    };
    // 已释放 AppState 锁
    ai_manager.some_method().await;
    Ok(())
}
```

### Step 2: 注册到 `invoke_handler!`

```rust
.invoke_handler(tauri::generate_handler![
    // ... 已有命令
    your_command,  // ← 加在这里
])
```

### Step 3: 前端调用

```typescript
import { invoke } from '@tauri-apps/api/core'

const result = await invoke<string>('your_command', { param1: 'value' })
```

### Step 4: 验证

```bash
cd src-tauri && cargo check
npx vue-tsc --noEmit
```
