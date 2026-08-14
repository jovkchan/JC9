<script setup lang="ts">
// 积木参数面板（Schema 驱动）：选中积木后编辑 config，见方案 §5.5
// 复用 BlockDef.fields 渲染，人工配置与 AI 生成落点一致
import { ref, watch, computed, onBeforeUnmount } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useAutomationStore } from '@/stores/automation'
import { getBlockDef, getBlockColor, getBlockLabel } from '@/components/automation/blocks/palette'
import type { BlockNode, FieldDef } from '@/types/automation'
import JcInput from '@/components/ui/JcInput.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcSwitch from '@/components/ui/JcSwitch.vue'
import JcButton from '@/components/ui/JcButton.vue'

const props = defineProps<{ node: BlockNode | null }>()
const emit = defineEmits<{ 'configure-credential': [] }>()
const store = useAutomationStore()

const draft = ref<Record<string, unknown>>({})
let timer = 0

watch(
  () => props.node?.id,
  () => { draft.value = { ...(props.node?.config ?? {}) } },
  { immediate: true },
)

/** 字段变更 → 更新草稿 → 防抖提交（一次停顿记一次历史） */
function set(key: string, v: unknown) {
  draft.value = { ...draft.value, [key]: v }
  clearTimeout(timer)
  timer = window.setTimeout(() => {
    if (props.node) store.updateNodeConfig(props.node.id, draft.value)
  }, 350)
}

/** 路径/文件选择器（Tauri plugin-dialog，写回对应字段）；浏览器/mock 下回退提示 */
async function pickField(f: FieldDef) {
  try {
    if (f.picker === 'file') {
      const d = await open({ multiple: false, title: `选择${f.label}`, filters: [{ name: f.label, extensions: ['*'] }] })
      if (typeof d === 'string' && d) set(f.key, d)
    } else {
      const d = await open({ directory: true, multiple: false, title: `选择${f.label}` })
      if (typeof d === 'string' && d) set(f.key, d)
    }
  } catch (e) {
    console.error('文件/目录选择不可用（需 Tauri 环境）', e)
  }
}

onBeforeUnmount(() => clearTimeout(timer))

const SHELL_OPTIONS = [
  { label: 'PowerShell', value: 'powershell' },
  { label: 'CMD', value: 'cmd' },
  { label: 'Bash', value: 'bash' },
  { label: 'SH', value: 'sh' },
  { label: 'Python', value: 'python' },
  { label: 'Node', value: 'node' },
]

const def = computed(() => (props.node ? getBlockDef(props.node.type) : undefined))
const color = computed(() => (props.node ? getBlockColor(props.node.type) : '#888'))
const label = computed(() => (props.node ? getBlockLabel(props.node.type) : '未选中'))

/** 工作积木选择（调用工作流）：下拉来自当前已保存的工作积木 */
const automationOptions = computed(() =>
  store.automations.map(a => ({ label: a.name || a.id, value: a.id })),
)

/** 调用工作流：目标积木的「手动触发」块选项（供「入口块 ID」选择，无需手填块 ID） */
const targetTriggerOptions = computed<{ label: string; value: string }[]>(() => {
  if (def.value?.type !== 'call-automation') return []
  const targetId = asString(draft.value['automationId'])
  if (!targetId) return []
  const target = store.automations.find(a => a.id === targetId)
  if (!target) return []
  return target.nodes
    .filter(n => n.type === 'manual-trigger')
    .map(n => ({
      label: n.config?.name ? `手动触发：${String(n.config.name)}` : `手动触发：${n.id.slice(0, 8)}…`,
      value: n.id,
    }))
})

function asString(v: unknown): string { return v == null ? '' : String(v) }
function asNumber(v: unknown): number { const n = Number(v); return Number.isFinite(n) ? n : 0 }

/** 按字段类型渲染控件 */
function fieldValue(f: FieldDef) {
  return draft.value[f.key]
}
</script>

<template>
  <div class="inspector">
    <template v-if="node">
      <div class="ins-head">
        <span class="ins-dot" :style="{ background: color }"></span>
        <span class="ins-label">{{ label }}</span>
      </div>
      <div class="ins-body">
        <!-- 凭据积木：配置凭据按钮（凭据 ID/名称由配置生成，不可手改） -->
        <div v-if="node.type === 'credential'" class="ins-cred-row">
          <JcButton size="small" type="primary" @click="emit('configure-credential')">配置凭据</JcButton>
          <span v-if="asString(draft['credentialName'])" class="ins-cred-name">{{ asString(draft['credentialName']) }}</span>
        </div>

        <div v-for="f in def?.fields ?? []" :key="f.key" class="ins-field">
          <label class="ins-f-label">
            {{ f.label }}
            <span v-if="f.required" class="ins-req">*</span>
          </label>

          <!-- 调用工作流：入口块（手动触发）下拉 + 粘贴兜底（无需手填块 ID） -->
          <div v-if="f.type === 'text' && f.key === 'entry' && targetTriggerOptions.length" class="ins-auto-row">
            <JcSelect
              :model-value="asString(fieldValue(f))"
              :options="targetTriggerOptions"
              placeholder="不填 = 用目标的「开始」"
              @update:model-value="(v) => set(f.key, String(v))"
            />
            <JcInput
              :model-value="asString(fieldValue(f))"
              placeholder="或粘贴块 ID（右键「复制块 ID」）"
              @update:model-value="(v) => set(f.key, String(v))"
            />
          </div>
          <!-- 文本（工作目录：可点选；凭据 ID：只读） -->
          <div v-else-if="f.type === 'text' || f.type === 'var'" class="ins-dir-row">
            <JcInput
              :model-value="asString(fieldValue(f))"
              :placeholder="f.placeholder"
              :disabled="f.key === 'credentialId'"
              @update:model-value="(v) => set(f.key, String(v))"
            />
            <JcButton v-if="f.picker" size="small" @click="pickField(f)">{{ f.picker === 'file' ? '选择文件' : '选择目录' }}</JcButton>
          </div>
          <!-- 多行文本 / 环境变量 -->
          <JcTextarea
            v-else-if="f.type === 'textarea' || f.type === 'env'"
            :model-value="asString(fieldValue(f))"
            :placeholder="f.placeholder"
            :rows="f.type === 'env' ? 3 : 4"
            @update:model-value="(v) => set(f.key, String(v))"
          />
          <!-- 数字 -->
          <input
            v-else-if="f.type === 'number'"
            class="ins-num"
            type="number"
            :value="asNumber(fieldValue(f))"
            :placeholder="f.placeholder"
            @input="set(f.key, (($event.target as HTMLInputElement).valueAsNumber) || 0)"
          />
          <!-- Shell -->
          <JcSelect
            v-else-if="f.type === 'shell'"
            :model-value="asString(fieldValue(f))"
            :options="SHELL_OPTIONS"
            @update:model-value="(v) => set(f.key, String(v))"
          />
          <!-- 工作积木选择（调用工作流）：下拉选已有 + 文本粘贴 ID 兜底 -->
          <div v-else-if="f.type === 'automation'" class="ins-auto-row">
            <JcSelect
              :model-value="asString(fieldValue(f))"
              :options="automationOptions"
              :placeholder="f.placeholder"
              @update:model-value="(v) => set(f.key, String(v))"
            />
            <JcInput
              :model-value="asString(fieldValue(f))"
              placeholder="或粘贴 ID（列表/编辑器右键复制）"
              @update:model-value="(v) => set(f.key, String(v))"
            />
          </div>
          <!-- 下拉 -->
          <JcSelect
            v-else-if="f.type === 'select'"
            :model-value="asString(fieldValue(f))"
            :options="(f.options ?? []).map(o => ({ label: String(o.label), value: o.value as string | number }))"
            @update:model-value="(v) => set(f.key, String(v))"
          />
          <!-- 开关 -->
          <JcSwitch
            v-else-if="f.type === 'switch'"
            :checked="!!fieldValue(f)"
            @update:checked="(v) => set(f.key, v)"
          />

          <div v-if="f.interpolatable" class="ins-tip">支持 &#123;&#123;变量&#125;&#125; / &#123;&#123;last.*&#125;&#125; 插值</div>
        </div>

        <div v-if="(def?.fields?.length ?? 0) === 0" class="ins-empty">该积木无可配置参数</div>

        <!-- 固定状态 -->
        <div class="ins-lock-row">
          <JcButton size="small" :type="node.locked ? 'primary' : 'default'" @click="store.toggleLock(node.id)">
            {{ node.locked ? '已固定' : '固定位置' }}
          </JcButton>
        </div>
      </div>
    </template>
    <div v-else class="ins-placeholder">
      <p>选中一个积木</p>
      <p class="ins-sub">在右侧编辑它的参数</p>
    </div>
  </div>
</template>

<style scoped lang="scss">
.inspector {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--jc-bg-panel, #252526);
  overflow: hidden;
}
.ins-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-bottom: 1px solid var(--jc-border-default, #3e3e42);
}
.ins-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.ins-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--jc-text-primary, #e6e6e6);
}
.ins-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.ins-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ins-f-label {
  font-size: 12px;
  color: var(--jc-text-secondary, #aaa);
}
.ins-req {
  color: var(--jc-color-error, #ff4d4f);
}
.ins-tip {
  font-size: 11px;
  color: var(--jc-text-tertiary, #858585);
}
.ins-num {
  width: 100%;
  height: 28px;
  padding: 0 8px;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong, #555);
  background: var(--jc-bg-input, #3c3c3c);
  color: var(--jc-text-primary, #e6e6e6);
  font-size: 12px;
  outline: none;
}
.ins-num:focus {
  border-color: var(--jc-color-accent, #8a58ff);
}
.ins-empty {
  font-size: 12px;
  color: var(--jc-text-tertiary, #858585);
  text-align: center;
  padding: 16px 0;
}
.ins-lock-row {
  padding-top: 4px;
  border-top: 1px dashed var(--jc-border-default, #3e3e42);
}
.ins-cred-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.ins-cred-name {
  font-size: 12px;
  color: var(--jc-color-success, #52c41a);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ins-dir-row {
  display: flex;
  gap: 6px;
  align-items: center;
}
.ins-dir-row > :first-child {
  flex: 1;
  min-width: 0;
}
.ins-auto-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.ins-auto-row > :first-child {
  width: 100%;
}
.ins-placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  color: var(--jc-text-tertiary, #858585);
  font-size: 13px;
}
.ins-sub {
  font-size: 12px;
}
</style>
