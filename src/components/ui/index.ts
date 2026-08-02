// ═══════════════════════════════════════════════════════════════
// JC9 UI 可移植组件库 —— 桶导出
//
// 用法：
//   import { JcButton, JcModal, toast, useJcTheme } from '@/components/ui'
//   import '@/components/ui/tokens.scss'   // 其他项目需要默认主题时引入
//
// 所有组件仅依赖 Vue 3 + TS，零业务依赖（不引 stores / Tauri / Pinia），
// 整个 ui/ 目录可整体复制到任意 Vue 3 项目使用。
// ═══════════════════════════════════════════════════════════════

// 组件
export { default as JcButton } from './JcButton.vue'
export { default as JcInput } from './JcInput.vue'
export { default as JcTextarea } from './JcTextarea.vue'
export { default as JcSelect } from './JcSelect.vue'
export { default as JcSegmented } from './JcSegmented.vue'
export { default as JcBadge } from './JcBadge.vue'
export { default as JcEmpty } from './JcEmpty.vue'
export { default as JcModal } from './JcModal.vue'
export { default as JcContextMenu } from './JcContextMenu.vue'
export { default as JcToast } from './JcToast.vue'
export { default as JcDropdown } from './JcDropdown.vue'
export { default as JcTooltip } from './JcTooltip.vue'
export { default as JcSwitch } from './JcSwitch.vue'
export { default as JcRadio } from './JcRadio.vue'
export { default as JcRadioGroup } from './JcRadioGroup.vue'
export { default as JcCheckbox } from './JcCheckbox.vue'
export { default as JcCheckboxGroup } from './JcCheckboxGroup.vue'
export { default as JcSkeleton } from './JcSkeleton.vue'
export { default as JcCard } from './JcCard.vue'
export { default as JcTable } from './JcTable.vue'
export { default as JcTree } from './JcTree.vue'
export { default as JcTabBar } from './JcTabBar.vue'
export type { JcTabItem } from './JcTabBar.vue'
export { default as ToolShell } from './ToolShell.vue'

// 命令式 Toast
export { toast, toastState, dismissToast, clearToasts } from './toast'
export type { JcToastType, JcToastItem } from './toast'

// 主题切换
export { useJcTheme, applyJcTheme, toggleJcTheme } from './theme'
export type { JcTheme } from './theme'

// 组件类型
export type { JcButtonType, JcButtonSize, JcButtonShape } from './JcButton.vue'
export type { JcInputSize } from './JcInput.vue'
export type { JcSelectOption, JcSelectSize } from './JcSelect.vue'
export type { JcSegmentedOption, JcSegmentedSize } from './JcSegmented.vue'
export type { JcBadgeStatus } from './JcBadge.vue'
export type { JcContextMenuItem } from './JcContextMenu.vue'
export type { JcDropdownItem } from './JcDropdown.vue'
export type { JcMenuItem } from './JcMenuList.vue'
export type { JcRadioOption } from './JcRadioGroup.vue'
export type { JcCheckboxOption } from './JcCheckboxGroup.vue'
export type { JcTableColumn } from './JcTable.vue'
export type { JcTreeNode } from './JcTree.vue'
