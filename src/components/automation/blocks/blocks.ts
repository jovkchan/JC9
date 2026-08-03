import type { BlockDef, Port } from '@/types/automation'

// 端口语义化稳定 id（AI 生成 edges 依赖，见方案 §5.3）
function flowIn(id = 'in'): Port {
  return { id, direction: 'in', color: '#8a58ff', dataType: 'flow' }
}
function flowOut(id = 'out'): Port {
  return { id, direction: 'out', color: '#8a58ff', dataType: 'flow' }
}

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
    type: 'command',
    category: 'terminal',
    label: '命令',
    color: '#8a58ff',
    inputs: [flowIn()],
    outputs: [flowOut()],
    fields: [
      { key: 'command', label: '命令', type: 'textarea', required: true, placeholder: '例如 npm run build', interpolatable: true },
      { key: 'shell', label: 'Shell', type: 'shell', default: 'powershell' },
      { key: 'cwd', label: '工作目录', type: 'text', placeholder: '{{cwd}} 或绝对路径', interpolatable: true },
      { key: 'env', label: '环境变量', type: 'env', placeholder: 'KEY=VALUE' },
      { key: 'timeoutSecs', label: '超时(秒)', type: 'number', default: 60 },
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
    type: 'end',
    category: 'end',
    label: '结束',
    color: '#ff4d4f',
    inputs: [flowIn()],
    outputs: [],
    fields: [],
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
