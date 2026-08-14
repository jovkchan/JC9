/**
 * 积木画布摘要 + 动态高度
 * 依据块类型与已配置内容生成 1..N 行摘要（显示在积木上），
 * 高度随摘要行数拉高（宽度一致 BLOCK_W）。编辑画布与运行时图共用。
 */
export const BLOCK_W = 200
/** 无摘要时的基础高度 */
const BASE_H = 60
/** 每行摘要额外高度（world 单位） */
const LINE_H = 14
/** 摘要最多行数（防止无限拉高） */
export const MAX_SUMMARY_LINES = 4

function s(v: unknown): string {
  return v == null ? '' : String(v).trim()
}
function nonEmptyLines(text: string, max: number): string[] {
  return text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter(Boolean)
    .slice(0, max)
}
function one(text: string): string[] {
  return text ? [text] : []
}

const STATUS_ACTIONS: Record<string, string> = { status: '变更清单', diff: '未提交差异', log: '最近提交' }
const BRANCH_ACTIONS: Record<string, string> = { checkout: '切换', create: '新建', delete: '删除', list: '列出', merge: '合并' }
const TAG_ACTIONS: Record<string, string> = { create: '创建', delete: '删除', list: '列出' }
const SHELL_NAMES: Record<string, string> = { powershell: 'PowerShell', bash: 'Bash', cmd: 'CMD' }
const JENKINS_ACT: Record<string, string> = { trigger: '触发', status: '查状态', console: '控制台' }
const GITLAB_ACT: Record<string, string> = { 'pipeline-trigger': '触发流水线', 'pipeline-status': '流水线状态', 'job-log': '任务日志', 'mr-create': '创建MR' }
const K8S_ACT: Record<string, string> = { apply: '应用', rollout: '回滚', get: '查看', logs: '日志' }
const DOCKER_ACT: Record<string, string> = { build: '构建', pull: '拉取', run: '运行', compose: 'Compose', images: '镜像列表', ps: '容器列表', logs: '日志', exec: '执行', stop: '停止', rm: '删除' }

/** 按块类型与配置生成摘要行（空数组 = 只有标题，无摘要区）
 * 核心内容（必填大字段）优先占行，附加配置（shell/超时/env/远程等）仅在非默认值且还有空间时追加。 */
export function blockSummary(type: string, config: Record<string, unknown>): string[] {
  const c = config ?? {}
  switch (type) {
    case 'command': {
      const rows = nonEmptyLines(s(c.command), 2)
      const sh = s(c.shell)
      if (sh && sh !== 'powershell') rows.push(`Shell ${SHELL_NAMES[sh] ?? sh}`)
      const to = s(c.timeoutSecs)
      if (to && to !== '0') rows.push(`超时 ${to}s`)
      const of = s(c.onFail)
      if (of && of !== 'stop') rows.push(`失败:${of === 'continue' ? '继续' : of}`)
      rows.push(...nonEmptyLines(s(c.env), 2).map((e) => `env ${e}`))
      return rows.slice(0, MAX_SUMMARY_LINES)
    }
    case 'open-url': {
      const rows = one(s(c.url))
      const b = s(c.browser)
      if (b && b !== 'default') rows.push(`浏览器 ${b}`)
      return rows
    }
    case 'launch': {
      const rows = one(s(c.program))
      if (s(c.args)) rows.push(s(c.args))
      if (c.wait === true) rows.push('等待完成')
      return rows
    }
    case 'workspace': {
      const rows = one(s(c.path))
      if (s(c.name)) rows.push(s(c.name))
      return rows
    }
    case 'env':
      return nonEmptyLines(s(c.env), MAX_SUMMARY_LINES)
    case 'condition': {
      const row = [s(c.left), s(c.op), s(c.right)].filter(Boolean).join(' ')
      return one(row)
    }
    case 'delay':
      return one(`${s(c.seconds)} 秒`)
    case 'loop': {
      if (c.mode === 'while') return one(`${s(c.left)} ${s(c.op)} ${s(c.right)}`)
      return one(`${s(c.count)} 次`)
    }
    case 'var-set': {
      const rows = one(`${s(c.varName)} = ${s(c.value)}`)
      const vt = s(c.varType)
      if (vt && vt !== 'string') rows.push(`类型 ${vt}`)
      return rows
    }
    case 'ai-generate': {
      const rows = nonEmptyLines(s(c.prompt), MAX_SUMMARY_LINES - 1)
      if (s(c.varName)) rows.push(`→ ${s(c.varName)}`)
      return rows
    }
    case 'notify': {
      const rows = one(s(c.title))
      if (s(c.body)) rows.push(s(c.body))
      return rows
    }
    case 'git-clone': {
      const rows = one(s(c.repo))
      if (s(c.dir)) rows.push(`→ ${s(c.dir)}`)
      if (s(c.branch)) rows.push(`-b ${s(c.branch)}`)
      return rows
    }
    case 'git-status':
      return one(`${STATUS_ACTIONS[s(c.action)] ?? s(c.action)}${s(c.path) ? ' ' + s(c.path) : ''}`)
    case 'git-commit': {
      const rows = nonEmptyLines(s(c.message), 3)
      if (c.addAll === false) rows.push('不暂存')
      return rows
    }
    case 'git-push':
      return one([s(c.remote) || 'origin', s(c.branch)].filter(Boolean).join(' '))
    case 'git-pull':
      return one([s(c.remote) || 'origin', s(c.branch)].filter(Boolean).join(' '))
    case 'git-branch':
      return one([BRANCH_ACTIONS[s(c.action)] ?? s(c.action), s(c.name)].filter(Boolean).join(' '))
    case 'git-tag': {
      const rows = one([TAG_ACTIONS[s(c.action)] ?? s(c.action), s(c.tag)].filter(Boolean).join(' '))
      if (s(c.message)) rows.push(s(c.message))
      return rows
    }
    case 'docker': {
      const rows = one(
        `${DOCKER_ACT[s(c.action)] ?? s(c.action)}${s(c.image) ? ' ' + s(c.image) : ''}${s(c.tag) && s(c.tag) !== 'latest' ? ':' + s(c.tag) : ''}`,
      )
      if (s(c.container)) rows.push(s(c.container))
      if (s(c.service)) rows.push(`svc ${s(c.service)}`)
      if (s(c.cmd)) rows.push(s(c.cmd))
      return rows.slice(0, MAX_SUMMARY_LINES)
    }
    case 'jenkins': {
      const rows = one(`${s(c.job)}${s(c.build) ? ' #' + s(c.build) : ''}`)
      const a = s(c.action)
      if (a && a !== 'trigger') rows.push(JENKINS_ACT[a] ?? a)
      if (s(c.url)) rows.push(s(c.url))
      return rows.slice(0, MAX_SUMMARY_LINES)
    }
    case 'harbor': {
      const rows = one(`${s(c.project)}/${s(c.repo)}:${s(c.tag) || 'latest'}`)
      if (s(c.context)) rows.push(s(c.context))
      if (s(c.dockerfile)) rows.push(s(c.dockerfile))
      return rows.slice(0, MAX_SUMMARY_LINES)
    }
    case 'k8s': {
      const rows = one([K8S_ACT[s(c.action)] ?? s(c.action), s(c.kind), s(c.name)].filter(Boolean).join(' '))
      if (s(c.namespace)) rows.push(`ns ${s(c.namespace)}`)
      if (s(c.file)) rows.push(s(c.file))
      return rows.slice(0, MAX_SUMMARY_LINES)
    }
    case 'gitlab': {
      const rows: string[] = []
      const a = s(c.action)
      if (a && a !== 'pipeline-trigger') rows.push(GITLAB_ACT[a] ?? a)
      if (s(c.project)) rows.push(s(c.project))
      if (s(c.ref)) rows.push(`ref ${s(c.ref)}`)
      if (s(c.jobId)) rows.push(`job #${s(c.jobId)}`)
      if (s(c.mrTitle)) rows.push(s(c.mrTitle))
      return rows.slice(0, MAX_SUMMARY_LINES)
    }
    case 'manual-trigger':
      return one(s(c.name))
    default:
      return []
  }
}

/** 块渲染高度（world 单位）：无摘要恒 BASE_H；有摘要贴合内容底部（摘要首行 + 行数 + 底部留白），不小于基础高 */
export function blockHeight(type: string, config: Record<string, unknown>): number {
  const lines = Math.min(blockSummary(type, config).length, MAX_SUMMARY_LINES)
  if (lines === 0) return BASE_H
  return Math.max(BASE_H, 32 + lines * LINE_H + 8)
}
