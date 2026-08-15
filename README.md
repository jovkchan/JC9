# JC9  - 极简项目与终端管理器

`jc9` 是一个专为开发者设计的现代、精致且高效的跨平台项目与终端管理器。基于 Rust Tauri v2 与 Vue 3 技术栈构建，通过集成底层的 portable-pty 支持，提供极致流畅且多窗口的命令行终端开发体验。除终端管理外，还集成了 **AI Agent 闭环生态、富文本笔记与知识库、记忆系统、内置 MCP Server 与积木式自动化编辑器**，是个人日常开发的「一体化工作台」。

项目只是为了我的工作方便，不喜勿喷，不喜勿喷，不喜勿喷。

##  特性

-  **智能项目感知**：自动扫描并检测 Go、Node.js / Vue / Tauri、Rust 等项目，智能匹配与推荐常用的编译及运行指令。
-  **全局快捷命令**：提供高频、常用与收藏命令的分类检索与一键调用，支持多平台文档在线查询。
-  **ConPTY/PTY 底层集成**：与 VS Code 和 WezTerm 同级的底层伪终端实现，无缝适配各种 CLI 交互。
-  **智能日志分析**：流式解码分析终端输出，实时汇总匹配 `ERROR`、`WARN`、`DEBUG` 及 `INFO` 等级别日志数据。
-  **双色主题系统**：自带高级黑金暗色主题及明亮白色主题，支持窗口拖拽及与系统高度同步的精细化无边框 TitleBar。
-  **工业级 AI Agent 闭环生态**：
   - **自愈式 ReAct 闭环**：Thought-Action-Observation 机制驱动 Worker 子代理自主修正编译、构建与运行报错。
   - **隐私保护与正则脱敏**：主动感知系统环境变量，正则检测并脱敏所有敏感密钥，确保密钥绝不离机。
   - **工作区只读白名单与越界拦截**：内置常用用户配置文件只读白名单直接放行，白名单外任意项目越界读写及高危命令一律硬性挂起并拦截。
   - **防轰炸聚合审批队列**：并发多代理运行时，多项高危操作统一推入前端聚合列表进行一键式红点批量审批，确保开发心流不被打断。
   - **硬死循环熔断断路器**：后端实时监控 Worker 状态，若触发连续重复报错或调用次数超限（15次）则立即强行熔断终止，杜绝 Token 成本暴涨。
   - **规约共享内存黑板**：基于严格 Schema 在多 Worker 间共享全局配置与 Bug 位置等结构化数据。
   - **多模式对话**：支持 CRAFT（全执行）/ ASK（只读问答）/ PLAN（规划模式）三种 Chat Mode，以及多会话管理、自动命名与历史持久化。
-  **富文本笔记编辑器**：基于 TipTap / Yiitap 移植的强大 WYSIWYG 编辑器，支持 Markdown / 纯文本 / 富文本多模式，含增强表格（合并拆分 / 底色 / 列宽）、Mermaid 图表、KaTeX 公式、图片、Callout、多列布局、版本历史（50 条快照）与撤销恢复、笔记链接（`jclink://`）与在线分享。
-  **本地记忆与知识库**：SQLite + sqlite-vec 本地向量库，混合语义搜索（向量 + FTS5 全文），支持笔记自动同步入库、Agent 记忆沉淀（`jc9_memory_*`）、按项目 scope 隔离与记忆压缩。
-  **内置 MCP Server**：自带 MCP Server，暴露笔记 / 记忆 / 自动化触发等 17 个工具，支持 **SSE**（`127.0.0.1:18899` + Bearer Token）与 **Stdio**（`node <释放的 jc9-mcp.mjs>`，运行时内嵌释放）两种方式被外部 MCP 客户端（Claude Desktop / VS Code 等）接入；同时内置 MCP 客户端可接入第三方 MCP 服务器。
-  **积木式自动化编辑器**：可视化 Canvas 拖拽编程，通过「开始 / 手动触发 / 命令 / 条件 / 延迟 / 循环 / 并行 / 变量 / 工作区 / 环境变量 / Git / Docker / GitLab / Jenkins / Harbor / K8S / 凭据 / AI 生成 / 通知 / 调用工作流 / 结束」等 30+ 积木块连线编排自动化工作流；内置 Rust 本地执行引擎（控制流 + 凭据加密 + 全量执行日志），支持 50 步撤销 / 重做、主题感知渲染与端口连线校验。
-  **自研可移植 UI 组件库**：`src/components/ui` 下自研一套对齐 Ant Design 设计规范（色彩 / 字体 / 8px 布局 / 暗黑模式 / 三级阴影）的 `Jc*` 组件体系（按钮 / 输入 / 选择 / 弹窗 / 右键菜单 / Toast 等 20+ 组件），全项目工具与表单已统一收编。

##  技术栈

- **前端核心**：Vue 3 + TypeScript + Pinia + Vite
- **终端渲染**：@xterm/xterm + @xterm/addon-fit
- **富文本编辑器**：@tiptap/* + @yiitap/vue + @yiitap/vue-preset（含 Yiitap 独立扩展包）
- **图表 / 数学**：mermaid + @mermaid-js/layout-elk + KaTeX
- **后端框架**：Rust + Tauri v2
- **终端桥接**：portable-pty（ConPTY/PTY）
- **数据库**：SQLite + sqlite-vec 向量扩展（FTS5 全文索引）

##  开始使用

### 前期准备

确保您的本地机器安装了以下环境：
- Node.js (>= 18.0.0)
- Rust 工具链 (Cargo, rustc)

### 安装依赖

在项目根目录下执行以下命令：

```bash
npm install
```

### 开发环境启动

启动开发模式，Tauri 将在后台编译 Rust 模块并唤起前端热重载窗口：

```bash
npm run dev
```

或者使用 Tauri CLI 直接开发：

```bash
npx tauri dev
```

### 构建生产包

打包生成高度压缩的可执行应用：

```bash
npm run build
```

或者：

```bash
npx tauri build
```

##  内置开发者实用工具箱 (Developer Toolbox)

`jc9` 内置了极其丰富的开箱即用开发者工具，100% 纯前端/本地实现，零外部依赖，支持拼音与关键词模糊检索：

-  **代码与格式化**
  - **JSON 格式化**：JSON 语句的一键美化缩进、压缩与语法校验。
  - **SQL 格式/压缩**：美化换行缩进与压缩传输，整理混乱的生产环境 SQL。
  - **代码对比 (Diff)**：文本与配置文件的双栏差异比对（最大支持 5000 行，智能高亮变更）。
  - **HTML 转义**：网页字符实体编码与解码（防止 XSS 调试必备）。
  - **正则测试器**：正则表达式的输入实时高亮匹配与正则语法测试。
  
-  **编码与加密 (Crypto & Encode)**
  - **命名风格转换**：下划线 `user_name`、驼峰 `userName`、帕斯卡 `UserName`、烤串 `user-name` 与大写常量风格的批量智能互转（保留代码缩进）。
  - **对称加解密 (AES/DES)**：支持 AES-ECB/CBC (128/192/256位)、DES-ECB/CBC 的多进制加解密调试。
  - **非对称加密 (RSA)**：RSA 密钥对生成、标准 PEM 证书导入导出、OAEP 加解密与 SHA256withRSA 签名/验签。
  - **哈希计算 (Hash)**：大文件拖拽分块流式读取（防内存溢出），支持 MD5、SHA-1、SHA-256、SHA-512 以及 **SM3 国密算法**。
  - **JWT 解码器**：一键解析 JSON Web Token 的 Header 与 Payload 荷载。
  - **Base64 / URL / Unicode 编解码**：文本/图片 Base64 互转、URL 编解码与 Unicode/ASCII 码点翻译转换。
  - **UUID 生成器**：UUID v1 / v4 批量生成，支持大写、去横线、自定义格式。
  - **图片 Base64**：本地图片拖拽转 Base64 数据（含预览与体积统计）。
  - **环境变量查看**：进程环境变量分类检索与实时过滤。

-  **排版与辅助**
  - **占位假文生成 (Lorem Ipsum)**：支持生成中英文、中国古典诗词及 HTML 标签的 UI 假文。
  - **文本行操作器**：多行文本去重、首尾修剪、空行过滤、自然数排序以及拼装合并（如 SQL `IN` 语句快速整理）。
  - **CSS 单位换算**：`px`、`rem`、`em` , `vw` , `vh` 双向联动换算，带物理大小滑块与缩放预览演示。
  - **SVG 预览与优化**：本地 SVG 导入/粘贴，透明棋盘格缩放预览，深度 DOM 级别冗余清理与浮点精度控制。
  - **二维码工具**：自定义 Logo、8种码眼/码点形状的二维码矢量生成与本地图片上传解析。
  - **颜色转换**：HEX / RGB / HSL / HSV / CMYK / LAB 多格式互转与调色预览。
  - **Markdown 转纯文本**：Markdown 源码一键转可读纯文本。
  - **图标生成**：文本 / Emoji 一键生成自定义尺寸与配色的 PNG 图标。

-  **系统与网络 (System & Network)**
  - **DNS 解析查询**：模拟 Linux `dig` 终端，支持 A/CNAME/MX/TXT/AAAA 记录的多源 DoH 查询。
  - **端口释放器**：一键检测并安全终止占用端口的僵死进程。
  - **时间戳 & 计算器**：Unix 时间戳互转，工作日日期偏移计算及毫秒差值计算。
  - **Cron 表达式生成**：点选式 Cron 自动生成，支持标准 Linux 与 Java Spring 双规范，智能 `?` 互斥。
  - **进制转换**：二、八、十、十六进制的高精度转换。
  - **SSH/SSL 证书生成**：自签名 SSL 证书对与多算法 SSH 密钥生成器。

##  主要功能模块

### 🧠 AI Agent 闭环生态

后端基于 Rust 实现（`src-tauri/src/ai/`，30+ 模块），前端提供多任务工作台与审批面板：

| 模块 | 说明 |
|------|------|
| ReAct 引擎 | Thought-Action-Observation 自愈闭环，Worker 自主排错 |
| Worker 调度器 | 10+ 并发隔离子代理，限流队列 + 状态快照 |
| Planner 规划器 | 任务树拆解（P0-P4 优先级 + 树形编号），CRAFT / ASK / PLAN 三模式 |
| 安全沙箱 | 环境变量正则脱敏、只读白名单放行、越界读写硬性拦截 |
| 审批队列 | 并发高危操作聚合审批，支持一键全部拒绝 |
| 熔断断路器 | 连续重复报错或调用超 15 次强制终止，防 Token 暴涨 |
| 共享黑板 | 严格 Schema 的多 Worker 结构化共享内存 |
| 知识库 / 记忆 | 语义搜索避坑笔记，自动沉淀与草稿箱小红点审阅 |
| 技能系统 | `.jc9/skills/` 技能文件启动时自动同步到知识库 |
| MCP 集成 | 内置 MCP Server + 客户端，可接入第三方 MCP |

### 📝 富文本笔记与知识库

- 基于 TipTap + Yiitap 移植的 WYSIWYG 编辑器（`src/components/notes/`、`src/components/editor/`、`src/extensions/`）
- 多模式编辑：Markdown / 纯文本 / 富文本，防转义污染的快照式切换
- 增强表格（合并 / 拆分 / 底色 / 列宽）、Mermaid、KaTeX、Callout、多列布局、Emoji 选择器、Bubble 浮动工具栏
- 版本历史（50 条全量快照 + 恢复前自动保险快照）、撤销 / 重做、`Ctrl+Enter` 保存
- 笔记链接（`jclink://note/ID`）、分组管理、在线分享、笔记自动同步知识库向量
- 记忆系统：`jc9_memory_*` 系列命令，按项目 scope 隔离，支持向量检索与压缩

### ⚙️ 积木式自动化编辑器

- 可视化 Canvas 拖拽编程（`src/components/automation/`），自研轻量画布引擎（平移 / 缩放 / 网格 / 吸附 / 框选 / 右键菜单）
- 积木块注册表（Schema 驱动，`src/types/automation.ts` + `blocks/blocks.ts`）：
  - 入口：开始 / 手动触发
  - 终端：命令（Shell 覆盖 PowerShell / CMD / Bash / Python / Node）/ 打开网址 / 启动程序
  - 逻辑：条件 / 延迟 / 循环（for / while）/ 并行组 / 调用工作流（防环深度上限）
  - 环境：工作区（链路 cwd）/ 环境变量
  - 变量：变量赋值（string / number / boolean）
  - 版本控制：Git 克隆 / 查看变更 / 提交 / 推送 / 拉取 / 分支 / 标签
  - 平台：Docker / GitLab / Jenkins / Harbor / K8S（CLI 执行 + 凭据连线注入）
  - 权限：凭据块（Basic / PAT / Token / SSH Key / Kubeconfig）
  - 其他：AI 生成 / 通知 / 结束
- 颜色匹配端口连线（同色可连、异色禁止）+ 贝塞尔曲线 + 箭头；凭据专属金色端口
- Rust 本地执行引擎（`src-tauri/src/automation.rs`）：链路上下文传递（cwd / shell / env / session / last）、`{{var}}` / `{{last.*}}` 插值、控制流（手动触发 / 并行 fork / 循环 / 可中断停止）、防死循环熔断、失败策略（stop / continue）
- 凭据 AES-256-GCM 加密存储（`~/.jc9/key.bin` 0600，前端仅掩码）；全量执行日志 `automation_logs.json`（每步耗时 / 退出码 / 鉴权 / 执行内容，保留 200 条）
- 独立 Store（50 步撤销 / 重做）、主题感知渲染、BlockPalette 积木面板 + InspectorPanel 参数面板 + LoginDialog 凭据登录
- MCP 外部触发：`jc9_automation_run` 按 ID 激活指定工作积木（列表 / 编辑器右键 → 复制 ID）

### 🔌 内置 MCP Server

- 暴露 **17 个工具**：笔记操作（8）+ 记忆操作（6）+ 诊断（2）+ 自动化触发（1），让外部 AI 读写 JC9 的笔记与记忆、并按 ID 触发工作积木
- 支持两种标准传输（对齐 MCP 接入配置规范）：**Stdio**（`command`/`args`/`env`）与 **SSE**（`url`/`headers`），均不含 `type` 字段
- **Stdio 方式**：`command`=`node`、`args` 指向**运行时释放的 `jc9-mcp.mjs`**（内嵌模板 → exe 同目录 `mcp/`，自动写入当前地址/端口），`env` 传 `key`；通过内置 MCP Server 读写笔记/记忆（需 JC9 运行中）
- **SSE 方式**：`http://127.0.0.1:18899/sse`（事件流）+ `http://127.0.0.1:18899/message`（同协议 HTTP POST 端点），`headers` 走 Bearer Token
- 三种端点均使用 API Key 做认证与权限隔离（scope + 分组白名单 + **工具白名单**，可对 17 个工具逐项开关，危险操作以红/黄/绿标识）；server 命名采用 kebab-case（`jc9` / `jc9-sse`）
- 知识库按 `project:{id}` 分组隔离；配置存 `settings` 表 `mcp_server_config` KV
- 笔记 CRUD 后通过 `notes:changed` 事件实时同步前端
