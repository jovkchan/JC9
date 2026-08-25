import type { BlockDef, Port } from '@/types/automation'

// 端口语义化稳定 id（AI 生成 edges 依赖，见方案 §5.3）
function flowIn(id = 'in'): Port {
  return { id, direction: 'in', color: '#8a58ff', dataType: 'flow' }
}
function flowOut(id = 'out'): Port {
  return { id, direction: 'out', color: '#8a58ff', dataType: 'flow' }
}
/** 失败分支端口（红）：块执行失败时走此端口（未连线则按 onFail 中止/继续） */
function failOut(): Port {
  return { id: 'out-fail', direction: 'out', color: '#ff4d4f', dataType: 'flow', multi: true }
}
/** 凭据端口（专属金色，dataType='credential'）：凭据块 out → 目标块 in */
function credPort(dir: 'in' | 'out', id: string): Port {
  return { id, direction: dir, color: '#faad14', dataType: 'credential' }
}

/** 凭据类型/平台选项（LoginDialog / InspectorPanel 共用） */
export const CRED_KIND_OPTIONS = [
  { label: '用户名+密码', value: 'basic' },
  { label: 'Personal Access Token', value: 'pat' },
  { label: 'API Token', value: 'token' },
  { label: 'SSH 私钥', value: 'ssh-key' },
  { label: 'Kubeconfig', value: 'kubeconfig' },
]
export const CRED_PLATFORM_OPTIONS = [
  { label: 'Docker', value: 'docker' },
  { label: 'GitLab', value: 'gitlab' },
  { label: 'Jenkins', value: 'jenkins' },
  { label: 'Harbor', value: 'harbor' },
  { label: 'K8S', value: 'k8s' },
  { label: 'SSH', value: 'ssh' },
]

/**
 * 积木注册表（Schema 驱动：InspectorPanel + AI 生成 + Canvas 渲染共用）
 * F1a 先注册基础积木；平台块 F5 注册（adapter.list_actions() → BlockDef）
 */
export const BLOCK_DEFS: BlockDef[] = [
  {
    type: 'start',
    category: 'entry',
    label: '开始',
    color: '#52c41a',
    inputs: [],
    outputs: [flowOut()],
    fields: [],
    compatRules: [],
  },
  {
    type: 'manual-trigger',
    category: 'entry',
    label: '手动触发',
    color: '#fa8c16',
    // 第二入口：无「开始」时从它启动；画布可对多个手动触发块分别触发各分支（F2）
    inputs: [],
    outputs: [flowOut()],
    fields: [
      { key: 'name', label: '触发名称', type: 'text', placeholder: '如 手动部署' },
    ],
    compatRules: [],
  },
  {
    type: 'command',
    category: 'terminal',
    label: '命令',
    color: '#8a58ff',
    inputs: [flowIn(), credPort('in', 'cred-in')],
    outputs: [flowOut(), failOut()],
    fields: [
      { key: 'command', label: '命令', type: 'textarea', required: true, placeholder: '例如 npm run build', interpolatable: true },
      { key: 'shell', label: 'Shell', type: 'shell', default: 'powershell' },
      // 工作目录由「工作区」块统一设置（链路 cwd），命令块无需配置
      { key: 'env', label: '环境变量', type: 'env', placeholder: 'KEY=VALUE', help: '设置子进程环境（区别于流程变量：流程变量用「变量赋值」+ {{var}} 插值；env 供命令运行时读取，如 PATH / CI_TOKEN）' },
      { key: 'timeoutSecs', label: '超时(秒)', type: 'number', default: 0, placeholder: '0 = 不限时' },
      { key: 'onFail', label: '失败策略', type: 'select', default: 'stop', options: [{ label: '停止', value: 'stop' }, { label: '继续', value: 'continue' }] },
    ],
    compatRules: [],
  },
  {
    type: 'condition',
    category: 'logic',
    label: '条件',
    color: '#4096ff',
    inputs: [flowIn()],
    outputs: [{ ...flowOut('out-true'), multi: true }, { ...flowOut('out-false'), multi: true }],
    fields: [
      { key: 'left', label: '左值', type: 'text', required: true, placeholder: '{{last.exitCode}} 或变量', interpolatable: true },
      { key: 'op', label: '比较', type: 'select', required: true, default: '==', options: [
        { label: '==', value: '==' }, { label: '!=', value: '!=' },
        { label: '>', value: '>' }, { label: '<', value: '<' },
        { label: '包含', value: 'contains' },
      ] },
      { key: 'right', label: '右值', type: 'text', required: true, interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'delay',
    category: 'logic',
    label: '延迟',
    color: '#ff9c6e',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'seconds', label: '秒', type: 'number', required: true, default: 5 },
    ],
    compatRules: [],
  },
  {
    type: 'loop',
    category: 'logic',
    label: '循环',
    color: '#fa541c',
    // 图模型（F2）：in → 进入循环；out → 循环体；循环体末连回 loop-in 则重复；结束后沿 done 走
    inputs: [flowIn('in'), flowIn('loop-in')],
    outputs: [flowOut('out'), flowOut('done')],
    fields: [
      { key: 'mode', label: '方式', type: 'select', default: 'for', options: [
        { label: '按次数', value: 'for' },
        { label: '按条件', value: 'while' },
      ] },
      { key: 'count', label: '次数', type: 'number', default: 3, interpolatable: true },
      { key: 'left', label: '条件左值', type: 'text', placeholder: '{{var}} / {{last.*}}', interpolatable: true },
      { key: 'op', label: '比较', type: 'select', default: '==', options: [
        { label: '==', value: '==' }, { label: '!=', value: '!=' },
        { label: '>', value: '>' }, { label: '<', value: '<' },
        { label: '包含', value: 'contains' },
      ] },
      { key: 'right', label: '条件右值', type: 'text', interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'parallel',
    category: 'logic',
    label: '并行',
    color: '#00b96b',
    // 图模型（F2）：in → branch（多出边=多分支并发）→ 全部完成后沿 join 汇合继续
    inputs: [flowIn()],
    outputs: [{ ...flowOut('branch'), multi: true }, flowOut('join')],
    fields: [],
    compatRules: [],
  },
  {
    type: 'var-set',
    category: 'variable',
    label: '变量赋值',
    color: '#36cfc9',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'varName', label: '变量名', type: 'var', required: true, placeholder: '如 BUILD_OK' },
      { key: 'varType', label: '类型', type: 'select', default: 'string', options: [
        { label: '字符串', value: 'string' }, { label: '数字', value: 'number' }, { label: '布尔', value: 'boolean' },
      ] },
      { key: 'value', label: '值', type: 'text', interpolatable: true, placeholder: '{{last.stdout}} 或常量' },
    ],
    compatRules: [],
  },
  {
    type: 'call-automation',
    category: 'logic',
    label: '调用工作流',
    color: '#eb2f96',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // 运行时把另一个工作积木作为子程序执行（共享父 ctx，日志并入父 RunLog；Rust 引擎防环 depth 上限）
    fields: [
      { key: 'automationId', label: '目标工作积木', type: 'automation', required: true, placeholder: '选择或粘贴工作积木 ID', interpolatable: true },
      { key: 'entry', label: '入口块 ID', type: 'text', placeholder: '可选：指定「手动触发」块 ID', interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'ai-generate',
    category: 'ai',
    label: 'AI 生成',
    color: '#722ed1',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // F4 AI 积木：自然语言描述需求 → AI 生成文本 → 写入变量（下游块 {{var}} 引用）
    fields: [
      { key: 'prompt', label: '需求', type: 'textarea', required: true, interpolatable: true, placeholder: '用自然语言描述，可引用 {{last.stdout}} / {{cwd}}，如：查看工作区 GIT 变更，生成一句中文提交信息' },
      { key: 'model', label: '模型', type: 'select', placeholder: '留空 = 默认模型', options: [], help: '从已配置的模型列表中选择；留空使用「设置 → AI」默认模型。若模型不支持 tools（如 GLM/vLLM），引擎会自动省略 tools 字段。' },
      { key: 'varName', label: '输出变量', type: 'var', placeholder: '如 COMMIT_MSG，留空则不保存' },
    ],
    compatRules: [],
  },
  {
    type: 'notify',
    category: 'notify',
    label: '通知',
    color: '#faad14',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // 统一通知通道：引擎发系统通知（官方插件，跨平台）+ emit notify 事件（前端 Toast / 通知中心）
    fields: [
      { key: 'title', label: '标题', type: 'text', required: true, placeholder: '如 构建完成', interpolatable: true },
      { key: 'body', label: '内容', type: 'textarea', placeholder: '通知正文，可引用 {{last.stdout}}', interpolatable: true },
      { key: 'level', label: '级别', type: 'select', default: 'info', options: [
        { label: '信息', value: 'info' },
        { label: '成功', value: 'success' },
        { label: '警告', value: 'warn' },
        { label: '错误', value: 'error' },
      ] },
    ],
    compatRules: [],
  },
  {
    type: 'workspace',
    category: 'env',
    label: '工作区',
    color: '#13c2c2',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // 环境块：设置链路上下文 cwd，下游命令块未指定工作目录时继承（见方案 §4.5）
    fields: [
      { key: 'path', label: '路径', type: 'text', required: true, placeholder: '工作区绝对路径', interpolatable: true, picker: 'dir' },
      { key: 'name', label: '名称', type: 'text', placeholder: '如 frontend 项目' },
    ],
    compatRules: [],
  },
  {
    type: 'env',
    category: 'env',
    label: '环境变量',
    color: '#597ef7',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // 环境块：设置一组链路环境变量，下游命令继承（可一行一个、多个命令共享）；命令块自身 env 叠加且覆盖
    fields: [
      { key: 'env', label: '变量（KEY=VALUE）', type: 'env', required: true, placeholder: '一行一个，如 NODE_ENV=production', interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'open-url',
    category: 'terminal',
    label: '打开网址',
    color: '#2f54eb',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // 单一职责：用系统默认（或指定）浏览器打开 URL；不执行程序
    fields: [
      { key: 'url', label: '网址', type: 'text', required: true, placeholder: 'https://...', interpolatable: true },
      { key: 'browser', label: '浏览器', type: 'select', default: 'default', options: [
        { label: '系统默认', value: 'default' },
        { label: 'Chrome', value: 'chrome' },
        { label: 'Edge', value: 'edge' },
        { label: 'Firefox', value: 'firefox' },
      ] },
    ],
    compatRules: [],
  },
  {
    type: 'launch',
    category: 'terminal',
    label: '启动程序',
    color: '#eb2f96',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // 单一职责：启动可执行程序 + 参数 + 工作目录；与「打开网址」分离
    fields: [
      { key: 'program', label: '程序', type: 'text', required: true, placeholder: 'exe / 命令路径', interpolatable: true, picker: 'file' },
      { key: 'args', label: '参数', type: 'text', placeholder: '空格分隔参数', interpolatable: true },
      { key: 'wait', label: '等待完成', type: 'switch', default: false },
    ],
    compatRules: [],
  },
  {
    type: 'end',
    category: 'end',
    label: '结束',
    color: '#ff4d4f',
    inputs: [flowIn()],
    outputs: [],
    fields: [],
    compatRules: [],
  },
  // ── GIT 积木组（通用，远端由 git remote / clone URL 决定，不绑定 GitHub/GitLab）──
  {
    type: 'git-clone',
    category: 'scm',
    label: 'Git 克隆',
    color: '#f05033',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'repo', label: '仓库地址', type: 'text', required: true, placeholder: 'git@gitlab:xx.git 或 https://...', interpolatable: true },
      { key: 'dir', label: '目标目录', type: 'text', placeholder: '留空 = 工作区', picker: 'dir' },
      { key: 'branch', label: '分支', type: 'text', placeholder: '可选' },
    ],
    compatRules: [],
  },
  {
    type: 'git-status',
    category: 'scm',
    label: 'Git 查看变更',
    color: '#f05033',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // 查看变更：输出到 {{last.stdout}} 供下游引用（如 AI 生成提交信息 / 变更说明）
    fields: [
      { key: 'action', label: '查看', type: 'select', default: 'status', options: [
        { label: '变更清单 (git status --short)', value: 'status' },
        { label: '未提交差异 (git diff)', value: 'diff' },
        { label: '最近提交 (git log -5 --oneline)', value: 'log' },
      ] },
      { key: 'path', label: '路径过滤', type: 'text', placeholder: '可选：仅该路径', interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'git-commit',
    category: 'scm',
    label: 'Git 提交',
    color: '#f05033',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'message', label: '提交信息', type: 'textarea', required: true, placeholder: '可由 AI 生成', interpolatable: true },
      { key: 'addAll', label: '全部暂存', type: 'switch', default: true },
    ],
    compatRules: [],
  },
  {
    type: 'git-push',
    category: 'scm',
    label: 'Git 推送',
    color: '#f05033',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'remote', label: '远端', type: 'text', default: 'origin', placeholder: 'origin' },
      { key: 'branch', label: '分支', type: 'text', placeholder: '留空 = 当前分支' },
    ],
    compatRules: [],
  },
  {
    type: 'git-pull',
    category: 'scm',
    label: 'Git 拉取',
    color: '#f05033',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'remote', label: '远端', type: 'text', default: 'origin', placeholder: 'origin' },
      { key: 'branch', label: '分支', type: 'text', placeholder: '留空 = 当前分支' },
    ],
    compatRules: [],
  },
  {
    type: 'git-branch',
    category: 'scm',
    label: 'Git 分支',
    color: '#f05033',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'action', label: '操作', type: 'select', default: 'checkout', options: [
        { label: '切换', value: 'checkout' },
        { label: '新建', value: 'create' },
        { label: '删除', value: 'delete' },
        { label: '列出', value: 'list' },
        { label: '合并', value: 'merge' },
      ] },
      { key: 'name', label: '分支名', type: 'text', interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'git-tag',
    category: 'scm',
    label: 'Git 标签',
    color: '#f05033',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'action', label: '操作', type: 'select', default: 'create', options: [
        { label: '创建', value: 'create' },
        { label: '删除', value: 'delete' },
        { label: '列出', value: 'list' },
      ] },
      { key: 'tag', label: '标签名', type: 'text', interpolatable: true },
      { key: 'message', label: '附注', type: 'text', placeholder: '可选' },
    ],
    compatRules: [],
  },
  // ── 平台积木（F5 起步：CLI 优先，凭据经凭据端口连线注入）──
  {
    type: 'jenkins',
    category: 'platform',
    label: 'Jenkins',
    color: '#d24939',
    inputs: [flowIn(), credPort('in', 'cred-in')],
    outputs: [flowOut()],
    fields: [
      { key: 'url', label: '平台地址', type: 'text', required: true, placeholder: 'http://jenkins:8080', interpolatable: true },
      { key: 'job', label: '任务名', type: 'text', required: true, interpolatable: true },
      { key: 'action', label: '操作', type: 'select', default: 'trigger', options: [
        { label: '触发构建', value: 'trigger' },
        { label: '查队列', value: 'queue' },
        { label: '查状态', value: 'status' },
        { label: '控制台输出', value: 'console' },
        { label: '停止构建', value: 'stop' },
      ] },
      { key: 'build', label: '构建号', type: 'text', placeholder: '查状态/控制台/停止用；空 = lastBuild', interpolatable: true },
      { key: 'params', label: '构建参数', type: 'env', placeholder: '触发用，KEY=VALUE 一行一个', help: '触发构建时作为 Jenkins 参数注入（buildWithParameters），如 DEPLOY_ENV=uat / VERSION=1.0.0', interpolatable: true },
      { key: 'tail', label: '控制台 N 行', type: 'number', default: 0, placeholder: '0 = 全部' },
    ],
    compatRules: [],
  },
  {
    type: 'harbor',
    category: 'platform',
    label: 'Harbor',
    color: '#60b932',
    inputs: [flowIn(), credPort('in', 'cred-in')],
    outputs: [flowOut()],
    fields: [
      { key: 'url', label: '平台地址', type: 'text', required: true, placeholder: 'https://harbor.example.com', interpolatable: true },
      { key: 'project', label: '项目', type: 'text', required: true, interpolatable: true },
      { key: 'repo', label: '仓库', type: 'text', required: true, interpolatable: true },
      { key: 'tag', label: '标签', type: 'text', default: 'latest', interpolatable: true },
      { key: 'context', label: '构建目录', type: 'text', placeholder: '留空 = 工作区', picker: 'dir' },
      { key: 'dockerfile', label: 'Dockerfile', type: 'text', placeholder: '留空 = 默认', picker: 'file' },
    ],
    compatRules: [],
  },
  {
    type: 'k8s',
    category: 'platform',
    label: 'K8S',
    color: '#326ce5',
    inputs: [flowIn(), credPort('in', 'cred-in')],
    outputs: [flowOut()],
    fields: [
      { key: 'action', label: '操作', type: 'select', default: 'apply', options: [
        { label: '应用清单', value: 'apply' },
        { label: '回滚状态', value: 'rollout' },
        { label: '更新镜像', value: 'set-image' },
        { label: '更新环境变量', value: 'set-env' },
        { label: '重启', value: 'restart' },
        { label: '查看', value: 'get' },
        { label: '日志', value: 'logs' },
      ] },
      { key: 'file', label: '清单文件', type: 'text', placeholder: 'apply 用（yaml 路径）', interpolatable: true, picker: 'file' },
      { key: 'kind', label: '资源类型', type: 'text', placeholder: '如 deployment / pods' },
      { key: 'name', label: '资源名', type: 'text', interpolatable: true },
      { key: 'namespace', label: '命名空间', type: 'text', interpolatable: true },
      { key: 'image', label: '镜像', type: 'text', placeholder: 'set-image 用，如 mes=reg/img:tag 或 *=img:tag', interpolatable: true },
      { key: 'env', label: '环境变量', type: 'text', placeholder: 'set-env 用，如 KEY=VALUE', interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'ssh',
    category: 'platform',
    label: 'SSH 远程命令',
    color: '#0ca5a5',
    inputs: [flowIn(), credPort('in', 'cred-in')],
    outputs: [flowOut()],
    // 远程执行命令：凭据端口接入 ssh-key（私钥存 password 栏，ssh -i）或 basic（用户名+密码，Windows 走 plink / 其他走 sshpass）
    fields: [
      { key: 'host', label: '主机', type: 'text', required: true, placeholder: '如 192.168.5.55', interpolatable: true },
      { key: 'port', label: '端口', type: 'number', default: 22 },
      { key: 'user', label: '用户', type: 'text', placeholder: '留空 = 凭据中的用户名', interpolatable: true },
      { key: 'auth', label: '认证方式', type: 'select', default: 'key', options: [
        { label: 'SSH 私钥', value: 'key' },
        { label: '用户名+密码', value: 'password' },
      ] },
      { key: 'command', label: '远程命令', type: 'textarea', required: true, placeholder: '如 systemctl status mes / ls / curl 本机健康检查', interpolatable: true },
      { key: 'timeoutSecs', label: '超时(秒)', type: 'number', default: 60, placeholder: '0 = 不限时' },
    ],
    compatRules: [],
  },
  {
    type: 'http',
    category: 'platform',
    label: 'HTTP 请求',
    color: '#fa541c',
    inputs: [flowIn(), credPort('in', 'cred-in')],
    outputs: [flowOut()],
    // 健康检查 / 通用请求：期望状态码匹配即成功；输出 http_code + 响应体到 {{last.stdout}}
    fields: [
      { key: 'method', label: '方法', type: 'select', default: 'GET', options: [
        { label: 'GET', value: 'GET' },
        { label: 'POST', value: 'POST' },
        { label: 'HEAD', value: 'HEAD' },
        { label: 'PUT', value: 'PUT' },
        { label: 'DELETE', value: 'DELETE' },
      ] },
      { key: 'url', label: '网址', type: 'text', required: true, placeholder: 'https://... 健康检查地址', interpolatable: true },
      { key: 'headers', label: '请求头', type: 'textarea', placeholder: '一行一个 Header: value', interpolatable: true },
      { key: 'body', label: '请求体', type: 'textarea', placeholder: 'POST/PUT 用', interpolatable: true },
      { key: 'expectCode', label: '期望状态码', type: 'text', default: '200', placeholder: '匹配即成功，如 200 / 2xx', interpolatable: true },
      { key: 'timeoutSecs', label: '超时(秒)', type: 'number', default: 15 },
    ],
    compatRules: [],
  },
  {
    type: 'docker',
    category: 'platform',
    label: 'Docker',
    color: '#0db7ed',
    inputs: [flowIn()],
    outputs: [flowOut()],
    // 通用本地 docker 操作（单一职责，不含 registry 登录；registry 推送用 Harbor 块）
    fields: [
      { key: 'action', label: '操作', type: 'select', default: 'build', options: [
        { label: '构建', value: 'build' },
        { label: '拉取', value: 'pull' },
        { label: '运行', value: 'run' },
        { label: 'Compose', value: 'compose' },
        { label: '镜像列表', value: 'images' },
        { label: '容器列表', value: 'ps' },
        { label: '日志', value: 'logs' },
        { label: '执行', value: 'exec' },
        { label: '停止', value: 'stop' },
        { label: '删除', value: 'rm' },
      ] },
      { key: 'image', label: '镜像', type: 'text', interpolatable: true },
      { key: 'tag', label: '标签', type: 'text', default: 'latest', interpolatable: true },
      { key: 'context', label: '构建目录', type: 'text', placeholder: '留空 = 工作区', picker: 'dir' },
      { key: 'dockerfile', label: 'Dockerfile', type: 'text', placeholder: '留空 = 默认', picker: 'file' },
      { key: 'container', label: '容器名', type: 'text', interpolatable: true },
      { key: 'service', label: 'Compose 服务', type: 'text', interpolatable: true },
      { key: 'cmd', label: '命令', type: 'text', placeholder: 'run/exec 用', interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'gitlab',
    category: 'platform',
    label: 'GitLab',
    color: '#fc6d26',
    inputs: [flowIn(), credPort('in', 'cred-in')],
    outputs: [flowOut()],
    // GitLab 服务操作（上传代码走 GIT 积木组）；需 PAT 凭据（Bearer）
    fields: [
      { key: 'url', label: '平台地址', type: 'text', required: true, placeholder: 'http://gitlab.example.com', interpolatable: true },
      { key: 'project', label: '项目 ID/路径', type: 'text', required: true, interpolatable: true },
      { key: 'action', label: '操作', type: 'select', default: 'pipeline-trigger', options: [
        { label: '触发流水线', value: 'pipeline-trigger' },
        { label: '流水线状态', value: 'pipeline-status' },
        { label: '任务日志', value: 'job-log' },
        { label: '创建 MR', value: 'mr-create' },
      ] },
      { key: 'ref', label: '分支/引用', type: 'text', placeholder: '流水线触发用', interpolatable: true },
      { key: 'jobId', label: '任务 ID', type: 'text', placeholder: '任务日志用', interpolatable: true },
      { key: 'mrSource', label: '源分支', type: 'text', placeholder: '建 MR 用', interpolatable: true },
      { key: 'mrTarget', label: '目标分支', type: 'text', placeholder: '建 MR 用', interpolatable: true },
      { key: 'mrTitle', label: 'MR 标题', type: 'text', placeholder: '建 MR 用', interpolatable: true },
    ],
    compatRules: [],
  },
  {
    type: 'credential',
    category: 'credential',
    label: '凭据',
    color: '#faad14',
    // 独立数据源积木：无流程端口；通过「凭据端口」连线被目标块引用（见方案 §6）
    inputs: [],
    outputs: [credPort('out', 'cred-out')],
    fields: [
      { key: 'credentialId', label: '凭据 ID', type: 'text', required: true, placeholder: '由「配置凭据」写入' },
      { key: 'credentialName', label: '凭据名称', type: 'text', placeholder: '如 生产 GitLab' },
    ],
    compatRules: [],
  },
  // ── 文件 / 变量增强（2026-08-25 release.ps1 编排）──
  {
    type: 'read-file',
    category: 'file',
    label: '读取文件',
    color: '#00b96b',
    inputs: [flowIn()],
    outputs: [flowOut(), failOut()],
    // 读文本文件（UTF-8）到变量/输出；路径相对 = 工作区（release.ps1 读 build.gradle 场景）
    fields: [
      { key: 'path', label: '文件路径', type: 'text', required: true, placeholder: '相对路径 = 工作区，如 app\\build.gradle', interpolatable: true, picker: 'file' },
      { key: 'varName', label: '输出变量', type: 'var', placeholder: '如 FILE_CONTENT，留空只写 stdout' },
    ],
    compatRules: [],
  },
  {
    type: 'write-file',
    category: 'file',
    label: '写入文件',
    color: '#00b96b',
    inputs: [flowIn()],
    outputs: [flowOut(), failOut()],
    // 写文本文件（UTF-8 无 BOM）；content 支持 {{var}} 插值
    fields: [
      { key: 'path', label: '文件路径', type: 'text', required: true, placeholder: '相对路径 = 工作区', interpolatable: true, picker: 'file' },
      { key: 'content', label: '内容', type: 'textarea', required: true, placeholder: '要写入的内容，可 {{变量}} 插值', interpolatable: true },
      { key: 'append', label: '追加模式', type: 'switch', default: false, help: '开 = 追加到末尾；关 = 覆盖' },
    ],
    compatRules: [],
  },
  {
    type: 'hash-file',
    category: 'file',
    label: '文件哈希',
    color: '#00b96b',
    inputs: [flowIn()],
    outputs: [flowOut(), failOut()],
    // 计算文件哈希（SHA-256/SHA-1），输出到变量 + {{last.stdout}}；替代手写 Get-FileHash
    fields: [
      { key: 'path', label: '文件路径', type: 'text', required: true, placeholder: '相对路径 = 工作区', interpolatable: true, picker: 'file' },
      { key: 'algorithm', label: '算法', type: 'select', default: 'sha256', options: [
        { label: 'SHA-256', value: 'sha256' },
        { label: 'SHA-384', value: 'sha384' },
        { label: 'SHA-512', value: 'sha512' },
      ] },
      { key: 'varName', label: '输出变量', type: 'var', placeholder: '如 APK_SHA256' },
    ],
    compatRules: [],
  },
  {
    type: 'capture',
    category: 'variable',
    label: '输出捕获',
    color: '#36cfc9',
    inputs: [flowIn()],
    outputs: [flowOut(), failOut()],
    // 按正则从文本（默认上一步输出）提取捕获组写多个变量；无匹配走失败分支
    fields: [
      { key: 'source', label: '输入文本', type: 'text', default: '{{last.stdout}}', placeholder: '默认取上一步输出 {{last.stdout}}', interpolatable: true },
      { key: 'pattern', label: '正则', type: 'textarea', required: true, placeholder: '含捕获组，如 versionCode\\s+(\\d+)', interpolatable: true },
      { key: 'vars', label: '捕获组=变量名', type: 'env', required: true, placeholder: '一行一个：1=CODE / 2=NAME / 0=整段匹配 / 命名组=变量' },
    ],
    compatRules: [],
  },
  {
    type: 'expr',
    category: 'variable',
    label: '运算',
    color: '#36cfc9',
    inputs: [flowIn()],
    outputs: [flowOut(), failOut()],
    // 数值运算（+ - * / % 括号）/ 版本号递增 / 文本替换；结果写变量 + stdout
    fields: [
      { key: 'mode', label: '方式', type: 'select', default: 'calc', options: [
        { label: '数值运算', value: 'calc' },
        { label: '版本号递增', value: 'bump' },
        { label: '文本替换', value: 'replace' },
      ] },
      { key: 'expr', label: '表达式', type: 'text', placeholder: 'calc：如 {{oldCode}} + 1 / ({{A}}+{{B}})*2', interpolatable: true },
      { key: 'version', label: '版本号', type: 'text', placeholder: 'bump：如 {{oldName}} 或 1.0.0', interpolatable: true },
      { key: 'text', label: '输入文本', type: 'textarea', placeholder: 'replace：要处理的文本，可 {{变量}} 插值', interpolatable: true },
      { key: 'pattern', label: '查找（正则）', type: 'text', placeholder: 'replace：正则，如 versionCode\\s+\\d+', interpolatable: true },
      { key: 'replacement', label: '替换为', type: 'text', placeholder: 'replace：可 {{变量}}，如 versionCode {{NEW_CODE}}', interpolatable: true },
      { key: 'part', label: '递增位', type: 'select', default: 'patch', options: [
        { label: '补丁 patch', value: 'patch' },
        { label: '次版本 minor', value: 'minor' },
        { label: '主版本 major', value: 'major' },
      ] },
      { key: 'varName', label: '输出变量', type: 'var', placeholder: '如 NEW_CODE / NEW_NAME' },
    ],
    compatRules: [],
  },
]

/** 由 FieldDef.default 生成初始 config */
export function defaultsFromFields(fields: BlockDef['fields']): Record<string, unknown> {
  const cfg: Record<string, unknown> = {}
  for (const f of fields) {
    if (f.default !== undefined) cfg[f.key] = f.default
  }
  return cfg
}
