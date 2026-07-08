import { ref, computed, nextTick } from 'vue'
import { useProjectStore } from '@/stores/project'

/**
 * 在终端执行选中文本的右键菜单状态管理器
 *
 * 用法：
 * ```ts
 * const { ctxShow, ctxStyle, ctxText, runningTerminals,
 *         openCtx, closeCtx, execInTerminal, createAndExec } = useExecInTerminal()
 * ```
 *
 * 模板：
 * ```html
 * <div @contextmenu="(e) => openCtx(e, selectedText)">
 *
 * <Teleport to="body">
 *   <div v-if="ctxShow" class="ctx-overlay" @click="closeCtx" @contextmenu.prevent="closeCtx">
 *     <div class="ctx-menu" :style="ctxStyle" @click.stop>
 *       ...
 *     </div>
 *   </div>
 * </Teleport>
 * ```
 */
export function useExecInTerminal() {
  const projectStore = useProjectStore()

  const ctxShow = ref(false)
  const ctxPos = ref({ x: 0, y: 0 })
  const ctxText = ref('')
  /** 经过防溢出计算后的最终样式 */
  const ctxStyle = ref({})

  const runningTerminals = computed(() => projectStore.getRunningTerminals())

  function openCtx(e: MouseEvent, text: string) {
    ctxText.value = text
    ctxPos.value = { x: e.clientX, y: e.clientY }
    ctxShow.value = true
    e.preventDefault()

    // 等 DOM 渲染后测量菜单高度，超出底部则上移
    nextTick(() => {
      const el = document.querySelector('.ctx-menu') as HTMLElement | null
      if (!el) return
      const menuH = el.offsetHeight
      const gap = 8
      let top = ctxPos.value.y
      if (top + menuH + gap > window.innerHeight) {
        top = window.innerHeight - menuH - gap
      }
      if (top < gap) top = gap
      ctxStyle.value = { left: ctxPos.value.x + 'px', top: top + 'px' }
    })
  }

  function closeCtx() {
    ctxShow.value = false
  }

  function execInTerminal(processId?: string) {
    const text = ctxText.value
    ctxShow.value = false
    if (text) projectStore.sendToTerminal(text, processId)
  }

  /** 新建一个终端再执行 */
  async function createAndExec() {
    const text = ctxText.value
    ctxShow.value = false
    if (!text) return
    const pid = await projectStore.startQuickTerminal()
    if (pid && text) projectStore.sendToTerminal(text, pid)
  }

  return { ctxShow, ctxStyle, ctxText, runningTerminals, openCtx, closeCtx, execInTerminal, createAndExec }
}
