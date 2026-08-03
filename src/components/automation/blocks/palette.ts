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
  { key: 'entry', label: '入口', blocks: ['start'] },
  { key: 'terminal', label: '终端', blocks: ['command'] },
  { key: 'logic', label: '逻辑', blocks: ['condition', 'delay'] },
  { key: 'variable', label: '变量', blocks: ['var-set'] },
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
