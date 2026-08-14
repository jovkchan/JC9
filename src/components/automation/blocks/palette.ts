import type { BlockCategory, BlockDef } from '@/types/automation'
import { BLOCK_DEFS } from './blocks'

export { BLOCK_DEFS, defaultsFromFields } from './blocks'

export interface PaletteSection {
  key: BlockCategory
  label: string
  /** 该分区下的积木 type 列表 */
  blocks: string[]
}

/** 积木分区（侧栏积木面板的展示顺序与分组，见方案 §4.1） */
export const PALETTE_SECTIONS: PaletteSection[] = [
  { key: 'entry', label: '入口', blocks: ['start', 'manual-trigger'] },
  { key: 'terminal', label: '终端', blocks: ['command', 'open-url', 'launch'] },
  { key: 'scm', label: '版本控制', blocks: ['git-clone', 'git-status', 'git-commit', 'git-push', 'git-pull', 'git-branch', 'git-tag'] },
  { key: 'platform', label: '平台', blocks: ['docker', 'jenkins', 'harbor', 'k8s', 'gitlab'] },
  { key: 'env', label: '环境', blocks: ['workspace', 'env'] },
  { key: 'logic', label: '逻辑', blocks: ['condition', 'delay', 'loop', 'parallel'] },
  { key: 'variable', label: '变量', blocks: ['var-set'] },
  { key: 'ai', label: 'AI', blocks: ['ai-generate'] },
  { key: 'notify', label: '通知', blocks: ['notify'] },
  { key: 'credential', label: '权限', blocks: ['credential'] },
  { key: 'end', label: '结束', blocks: ['end'] },
]

const BY_TYPE = new Map<string, BlockDef>(BLOCK_DEFS.map(d => [d.type, d]))

export function getBlockDef(type: string): BlockDef | undefined {
  return BY_TYPE.get(type)
}

export function getBlockLabel(type: string): string {
  return getBlockDef(type)?.label ?? type
}

export function getBlockColor(type: string): string {
  return getBlockDef(type)?.color ?? '#8a58ff'
}
