// ── 积木帮助系统（侧栏「帮助」标签页数据）──
// 随积木注册表更新：新增积木时在此补条目，并更新下方「缺失/规划」清单。
// 依据设计文档 §4.1 分区与 §4.5 上下文模型。

export interface BlockHelp {
  type: string
  /** 可以做什么 */
  usage: string
  /** 什么情况下可以使用 */
  when: string
  /** 下游可以关联哪些 */
  downstream: string
  /** 可以如何组合（示例） */
  combos: string[]
}

/** 已注册积木的帮助（顺序按调色板） */
export const BLOCK_HELP: Record<string, BlockHelp> = {
  start: {
    type: 'start',
    usage: '自动化流程的唯一起点。每个任务有且仅有一个「开始」块，从这里沿连线向下游依次执行。',
    when: '创建任何新任务都必须先放置「开始」；它是整个流程的入口，画布校验要求有且仅有一个。',
    downstream: '输出「流程」端口（紫）→ 可连接任意带流程入端口的积木：命令、条件、延迟、变量赋值、结束；可扇出多条线并行启动多个分支。',
    combos: [
      '开始 → 命令 → 结束',
      '开始 → 条件 →（满足→命令 / 否则→命令）→ 结束',
      '开始 ⤵ 命令A / 命令B（并行）→ 结束',
    ],
  },
  'manual-trigger': {
    type: 'manual-trigger',
    usage: '一个可手动点击启动的分支入口。不设「开始」块时作为任务起点；同一任务可放多个「手动触发」，画布分别触发各子流程。',
    when: '需要把一个大任务拆成多个可单独手动启动的步骤/子流程时。',
    downstream: '输出「流程」端口（紫）→ 连接该子流程的第一个积木。',
    combos: ['手动触发 → 命令：手动执行一段操作', '手动触发（A）→ 命令A / 手动触发（B）→ 命令B：同一任务两个手动入口'],
  },
  command: {
    type: 'command',
    usage: '在本地执行 shell 命令（默认 PowerShell，可切 CMD / Bash / Python / Node）。工作目录由「工作区」块统一设置（链路 cwd），本块无需配置；「环境变量」设置子进程环境（与流程变量不同）；超时 0 = 不限时（大型编译不设限，需要时手动填秒数）。',
    when: '需要运行构建、脚本、测试、文件操作等任何本地命令时；是终端操作的核心积木。',
    downstream: '流程 out → 任意带流程 in 的积木；可用 {{last.exitCode}} 接条件判断成败；金色「凭据 in」端口可接入凭据积木注入登录环境变量。',
    combos: [
      '工作区 → 命令：命令在工作区路径下执行',
      '命令 → 条件：判断 exitCode（== 0 成功）',
      '命令 → 变量赋值：把 {{last.stdout}} 捕获成变量',
      '凭据 → 命令：注入 CI_TOKEN 等凭据到环境变量',
    ],
  },
  condition: {
    type: 'condition',
    usage: '分支判断（== / != / > / < / 包含），按结果走「满足条件 / 否则」两个分支之一（两分支端口为琥珀色，可各自连多条线）。',
    when: '需要根据上一块结果或变量值决定不同执行路径时。',
    downstream: 'out-true / out-false 各接一条分支，可分别连任意流程积木；两条分支可汇合到同一积木。',
    combos: [
      '命令 → 条件 →（满足→发布 / 否则→重试）→ 结束',
      '条件 → 变量赋值：按分支写入不同变量',
    ],
  },
  delay: {
    type: 'delay',
    usage: '等待指定秒数后再继续执行。',
    when: '需要等待服务就绪、限流、错开执行时机时。',
    downstream: 'out → 任意带流程 in 的积木。',
    combos: [
      '命令 → 延迟 → 命令：等待上一命令生效',
      '延迟 → 命令：定时/延时触发',
    ],
  },
  loop: {
    type: 'loop',
    usage: '按次数（for）或按条件（while）重复执行循环体。out 端口连循环体；循环体末端连回本块的「loop-in」端口即可重复；循环结束后沿 done 端口继续。',
    when: '需要对同一批命令重复执行 N 次，或根据条件反复处理直到满足时。',
    downstream: 'out（紫）→ 循环体第一个积木；loop-in（紫，入端口）← 循环体最后一个积木；done（紫）→ 循环结束后继续的积木。',
    combos: [
      '循环 → 命令 → 循环(loop-in)：重复执行命令 N 次',
      '循环(while) → 变量赋值 → 循环(loop-in)：条件循环（改变量让条件收敛）',
      '循环 → 并行 → 循环(loop-in)：每轮并发处理',
    ],
  },
  parallel: {
    type: 'parallel',
    usage: '并行组：branch 端口的多条出边作为多个分支并发执行，全部完成后沿 join 端口汇合继续。',
    when: '多个互不依赖的步骤希望同时执行（如同时构建前后端、同时查多个服务）以节省时间时。',
    downstream: 'branch（琥珀多边，紫）→ 各分支起点；join（紫）→ 全部完成后继续的积木。',
    combos: ['并行 ⤵ 命令A / 命令B ⤴ join → 结束', '并行 ⤵ 命令A / 命令B ⤴ join → 循环：每轮并发'],
  },
  'call-automation': {
    type: 'call-automation',
    usage: '调用另一个工作积木（子工作流）——运行时把它作为子程序执行，共享当前变量/工作目录/环境（子程序内改动会写回父流程）。目标选「工作积木」下拉或粘贴 ID（列表/编辑器右键「复制 ID」）；可选指定「手动触发」入口块 ID。',
    when: '把大任务拆成可复用的小工作积木、多处需要执行同一段流程时；工作积木可当「函数库」被复用，配合外部触发（MCP）可被任意处调用。',
    downstream: '流程 out → 任意带流程 in 的积木；子工作流执行完后继续向下。',
    combos: [
      '命令 → 调用工作流（复用「打包并发布」）→ 通知',
      '循环 → 调用工作流：每轮调用同一子流程处理一批',
      '条件（成功）→ 调用工作流 → 结束',
    ],
  },
  'var-set': {
    type: 'var-set',
    usage: '写入一个变量，供下游通过 {{变量名}} 插值引用；值可引用上一块输出（{{last.stdout}} 等）。',
    when: '需要跨多个积木传值、或把命令输出保存为变量时。',
    downstream: '流程 out → 任意带流程 in 的积木；变量本身通过插值被下游引用（无需连线）。',
    combos: [
      '命令 → 变量赋值（值={{last.stdout}}）→ 命令（引用 {{VAR}}）',
      '条件 → 变量赋值：写入分支结果',
    ],
  },
  end: {
    type: 'end',
    usage: '流程终点，表示任务成功完成；各分支最终汇聚到这里。',
    when: '每个任务结尾放置；多分支时各支路都应连到结束。',
    downstream: '无输出端口（流程到此结束）。',
    combos: ['任意流程末端 → 结束'],
  },
  credential: {
    type: 'credential',
    usage: '独立数据源积木——保存一份登录凭据（用户名/密码、Token、SSH 私钥、Kubeconfig），通过金色「凭据端口」连线被目标积木引用。',
    when: '目标积木需要登录/鉴权（如远程命令、未来的平台操作）时；一条凭据可连多个目标。',
    downstream: '凭据 out（金）→ 目标积木的「凭据 in」（金端口），如命令积木；可同时连多个目标。',
    combos: [
      '凭据 → 命令：注入 CI_TOKEN 等凭据到环境变量',
      '凭据 →（未来）Docker / GitLab / Jenkins / Harbor / K8S 平台积木',
    ],
  },
  workspace: {
    type: 'workspace',
    usage: '环境块——设置链路工作目录，下游所有命令块在未单独指定工作目录时自动继承。',
    when: '需要把后续命令统一限定在某个项目目录（如拉取/构建/部署都在同一仓库目录）时；放在流程靠前位置。',
    downstream: '流程 out → 任意带流程 in 的积木；其 cwd 上下文对其后整条链路生效（直到被再次覆盖）。',
    combos: [
      '工作区 → 命令：在该目录执行 git clone / build',
      '工作区 → 启动程序：程序默认在该目录运行',
      '工作区 →（未来）GitLab / Jenkins 等平台积木',
    ],
  },
  env: {
    type: 'env',
    usage: '环境块——设置一组链路环境变量（一行一个 KEY=VALUE），下游所有命令块自动继承；命令块自身「环境变量」字段可叠加并覆盖。',
    when: '多个命令需要共享同一组环境变量（如 NODE_ENV、CI_*、私有源地址）时，避免在每条命令里重复填写。',
    downstream: '流程 out → 任意带流程 in 的积木；其 env 上下文对其后整条链路生效（直到被再次覆盖）。',
    combos: [
      '环境变量 → 命令 → 命令：一组变量供多条命令共享',
      '环境变量（NODE_ENV=production）→ 命令：npm run build 读到生产环境',
    ],
  },
  'open-url': {
    type: 'open-url',
    usage: '用系统默认（或指定 Chrome/Edge/Firefox）浏览器打开一个网址。',
    when: '需要查看部署结果、打开 CI 页面、跳转文档时；单一职责，不执行本地程序。',
    downstream: '流程 out → 任意带流程 in 的积木（打开后通常接结束）。',
    combos: [
      '命令（部署成功）→ 打开网址：打开线上地址验证',
      'K8S 部署 → 打开网址：打开服务入口',
    ],
  },
  'ai-generate': {
    type: 'ai-generate',
    usage: 'AI 块——用自然语言描述需求，由已配置的模型生成结果文本；可引用上一块输出（{{last.stdout}}）、工作目录（{{cwd}}）等插值。生成结果写入「输出变量」供下游 {{变量}} 引用，同时作为本块输出。',
    when: '需要根据上下文动态生成内容（提交信息、变更说明、脚本片段、错误分析）时；需先在「设置 → AI」配置模型，未配置时运行会报错提示。',
    downstream: '流程 out → 任意带流程 in 的积木；生成文本通过变量插值被下游引用（无需连线）。',
    combos: [
      '工作区 → Git 查看变更 → AI 生成（参考 {{last.stdout}} 生成提交信息→写 COMMIT_MSG）→ Git 提交（{{COMMIT_MSG}}）',
      '命令（失败输出）→ AI 生成（分析 {{last.stdout}} 给出修复建议）→ 打开网址',
      '变量赋值（{{last.stdout}}）→ AI 生成（改写 / 摘要）→ 变量赋值（{{AI_OUT}}）',
    ],
  },
  notify: {
    type: 'notify',
    usage: '发送系统级通知（跨平台 Windows/macOS/Linux，走官方通知插件）+ 应用内 Toast 与通知中心。标题/内容可插值 {{变量}}/{{last.*}}，级别可选信息/成功/警告/错误。',
    when: '自动化执行到关键节点（构建完成/部署成功/任务失败）需要主动提醒用户时，或需要把结果沉淀到通知中心留痕时。',
    downstream: '流程 out → 任意带流程 in 的积木；通知不影响变量与后续执行，仅做用户反馈。',
    combos: [
      '命令（构建完成）→ 通知（标题「构建完成」，级别「成功」）→ 结束',
      '条件（失败）→ 通知（级别「错误」，内容提示失败原因）→ 结束',
      '工作区 → 命令 → 通知（标题/内容插值 {{last.stdout}}）',
    ],
  },
  launch: {
    type: 'launch',
    usage: '启动一个可执行程序（exe/命令），程序可点选文件；工作目录由「工作区」块统一设置（链路 cwd）；默认不等待其结束，可勾选等待完成。',
    when: '需要拉起本机程序（如打包工具、编辑器、PDF 阅读器）时；与「打开网址」分离，各司其职。',
    downstream: '流程 out → 任意带流程 in 的积木。',
    combos: [
      '命令（打包完成）→ 启动程序：用指定工具打开产物',
      '启动程序（wait 开启）→ 命令：等待工具跑完再继续',
      '工作区 → 启动程序：程序在项目目录运行',
    ],
  },
  'git-clone': {
    type: 'git-clone',
    usage: '把远程仓库克隆到本地（工作区）。远端地址决定走哪个托管平台（GitLab/GitHub/Gitee 均可），GIT 积木组完全通用。',
    when: '任务开始时需要拿到仓库代码；指定目标目录与可选分支。',
    downstream: '流程 out → 任意带流程 in 的积木（通常接 git-status / git-commit / 命令 / git-branch）。',
    combos: [
      '工作区 → Git 克隆（克隆到工作区）→ 命令（构建）',
      'Git 克隆（-b dev）→ Git 分支 → Git 提交',
    ],
  },
  'git-status': {
    type: 'git-status',
    usage: '查看工作区变更（默认 status --short 简洁清单），把变更内容输出到 {{last.stdout}}，供下游引用（如 AI 生成提交信息）。可选查看未提交差异（diff）或最近提交（log）。',
    when: '提交前需要先知道改了什么；是「AI 生成提交信息」「生成变更说明 / 审查差异」等场景的前置步骤。',
    downstream: '流程 out → 任意带流程 in 的积木；变更文本经 {{last.stdout}} 被下游引用（无需连线）。',
    combos: [
      '工作区 → Git 查看变更 → AI 生成（参考 {{last.stdout}} 写提交信息→COMMIT_MSG）→ Git 提交（{{COMMIT_MSG}}）',
      'Git 克隆 → Git 查看变更（差异）→ 命令：审查差异',
    ],
  },
  'git-commit': {
    type: 'git-commit',
    usage: '暂存改动（默认全部）并提交。提交信息可手动填写，也可由「AI 生成」积木产出后经插值填入。',
    when: '代码/文件有改动需要落库时；是「上传到远端」的前置步骤。',
    downstream: '流程 out → 任意带流程 in 的积木（通常接 git-push）。',
    combos: [
      '命令（改代码）→ Git 提交 → Git 推送',
      '工作区 → Git 查看变更 → AI 生成（参考 {{last.stdout}} 生成提交信息→COMMIT_MSG）→ Git 提交（{{COMMIT_MSG}}）→ Git 推送',
      '工作区 → Git 提交 → Git 推送 →（未来）触发 GitLab Pipeline',
    ],
  },
  'git-push': {
    type: 'git-push',
    usage: '把本地提交推送到远端（默认 origin）。',
    when: '需要把代码同步到远端仓库时；配合 git-commit 完成「上传代码」。',
    downstream: '流程 out → 任意带流程 in 的积木（如打开网址看 Pipeline、触发 Jenkins）。',
    combos: [
      'Git 提交 → Git 推送 → 打开网址（远端提交页）',
      'Git 推送 →（未来）Jenkins / GitLab Pipeline 触发',
    ],
  },
  'git-pull': {
    type: 'git-pull',
    usage: '拉取远端最新代码（默认 origin）。',
    when: '开始任务前需要同步远端他人改动时。',
    downstream: '流程 out → 任意带流程 in 的积木。',
    combos: ['Git 拉取 → 命令（构建最新代码）', '工作区 → Git 拉取 → Git 分支'],
  },
  'git-branch': {
    type: 'git-branch',
    usage: '分支操作：切换 / 新建 / 删除 / 列出 / 合并。',
    when: '按分支开发、切到发布分支、合并 feature 分支时。',
    downstream: '流程 out → 任意带流程 in 的积木（通常接 git-commit / git-push）。',
    combos: [
      'Git 分支（新建 feature）→ 命令 → Git 提交',
      'Git 分支（合并）→ Git 推送',
    ],
  },
  'git-tag': {
    type: 'git-tag',
    usage: '版本标签：创建（可带附注）/ 删除 / 列出。',
    when: '发版时打版本号、标记里程碑时。',
    downstream: '流程 out → 任意带流程 in 的积木（通常接 git-push 推送标签）。',
    combos: ['Git 提交 → Git 标签（v1.0.0）→ Git 推送', 'Git 标签（列出）→ 命令'],
  },
  jenkins: {
    type: 'jenkins',
    usage: 'Jenkins 服务操作：触发构建 / 查构建状态 / 拉控制台输出。需通过凭据端口接入「用户名+API Token」凭据（自动处理 Crumb CSRF）。',
    when: '代码推上去后触发 CI 构建、或等待构建结果时。',
    downstream: '流程 out → 任意带流程 in 的积木（通常接 harbor / k8s / 打开网址）。',
    combos: [
      'Git 推送 → Jenkins（触发构建）→ Jenkins（查状态）',
      'Jenkins（成功）→ Harbor（打镜像）',
      'Jenkins（控制台输出）→ 条件（判断构建结果）',
    ],
  },
  harbor: {
    type: 'harbor',
    usage: 'Harbor 镜像仓库：登录 + 构建 + 推送镜像（docker build/push 到 Harbor）。需凭据端口接入「用户名+密码」凭据。',
    when: '构建产物要制成镜像并存入 Harbor 时。',
    downstream: '流程 out → 任意带流程 in 的积木（通常接 k8s 部署）。',
    combos: ['Jenkins（打包完成）→ Harbor（打镜像）→ K8S（部署）', 'Git 推送 → Harbor → K8S'],
  },
  k8s: {
    type: 'k8s',
    usage: 'Kubernetes 部署：应用清单 / 回滚状态 / 查看资源 / 拉日志（kubectl）。需凭据端口接入「Kubeconfig」凭据。',
    when: '镜像已推送到 Harbor 后部署到集群、或查看部署状态时。',
    downstream: '流程 out → 任意带流程 in 的积木（通常接打开网址验证）。',
    combos: [
      'Harbor（镜像就绪）→ K8S（apply）→ K8S（rollout）',
      'K8S（部署成功）→ 打开网址（线上入口）',
      'K8S（get/logs）→ 条件',
    ],
  },
  docker: {
    type: 'docker',
    usage: '通用本地 Docker 操作：构建 / 拉取 / 运行 / Compose / 镜像与容器列表 / 日志 / 执行 / 停止 / 删除。',
    when: '需要构建镜像、起容器、查看容器状态或执行容器内命令时；单一职责，不含 registry 登录（推送镜像用 Harbor 块）。',
    downstream: '流程 out → 任意带流程 in 的积木。',
    combos: [
      'Docker（build）→ Docker（run）→ Docker（logs）',
      '命令（写 Dockerfile）→ Docker（build）→ Harbor（push）',
      'Docker（compose up）→ 打开网址（服务入口）',
    ],
  },
  gitlab: {
    type: 'gitlab',
    usage: 'GitLab 服务操作：触发流水线 / 查流水线状态 / 拉任务日志 / 创建 MR。需凭据端口接入「PAT」凭据（PRIVATE-TOKEN）。',
    when: '代码推送后触发 CI、查看 Pipeline 结果、或提交合并请求时（上传代码本身走 GIT 积木组）。',
    downstream: '流程 out → 任意带流程 in 的积木。',
    combos: [
      'Git 推送 → GitLab（触发流水线）→ GitLab（查状态）',
      'GitLab（流水线成功）→ Harbor（打镜像）',
      'Git 分支 → GitLab（创建 MR）',
    ],
  },
}

/** 规划中 / 缺失积木（对照设计文档 §4.1，随进度补建） */
export interface PlannedBlock {
  section: string
  name: string
  note: string
  milestone?: string
}

export const PLANNED_BLOCKS: PlannedBlock[] = [
  // 「外部触发（MCP）」已实现（F7 2026-08-14）；「调用工作流」已实现（F7 2026-08-14：call-automation 块，共享父 ctx + depth 防环）
  { section: '平台', name: 'GitLab', note: '服务操作已建；可扩展 查制品/仓库管理 等更多动作', milestone: 'F5' },
  { section: '平台', name: 'Docker', note: '已建通用操作；可扩展 registry 登录（现由 Harbor 承担）' },
  { section: '环境', name: '路径解析', note: '解析 {{var}} / 相对路径' },

  { section: '逻辑', name: '错误处理', note: '捕获失败并走降级路径' },
  { section: '变量', name: '变量读取', note: '在任意位置引用变量（当前靠插值）' },
  { section: '变量', name: '运算', note: '数值/字符串运算后写回' },
  { section: '变量', name: '输出捕获', note: '按正则从 stdout 捕获片段到变量' },
  { section: '结束', name: '失败结束', note: '流程异常终止（当前结束=成功）' },
]
