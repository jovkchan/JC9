<script setup lang="ts">
// 「配置凭据」弹窗：为画布上的凭据积木选择/新建凭据（独立数据源，见方案 §6）
// 绑定结果写 node.config.credentialId / credentialName；目标块通过「凭据端口」连线引用
import { ref, watch, computed } from 'vue'
import { useAutomationStore } from '@/stores/automation'
import type { BlockNode, CredentialKind } from '@/types/automation'
import JcModal from '@/components/ui/JcModal.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcSelect from '@/components/ui/JcSelect.vue'
import JcButton from '@/components/ui/JcButton.vue'

const props = defineProps<{ open: boolean; node: BlockNode | null }>()
const emit = defineEmits<{ 'update:open': [v: boolean] }>()
const store = useAutomationStore()

// 当前凭据块已绑定的凭据
const bound = computed<{ credentialId: string; credentialName: string } | null>(() => {
  const c = props.node?.config
  const id = c?.credentialId
  const name = c?.credentialName
  if (!id) return null
  return { credentialId: String(id), credentialName: name ? String(name) : '（未命名凭据）' }
})

// ── 选择已有凭据 ──
const selectedId = ref('')
const credentialOptions = computed(() =>
  store.credentials.map(c => ({ label: `${c.name}（${c.platform}/${c.kind}）`, value: c.id })),
)

function bindExisting() {
  if (!props.node || !selectedId.value) return
  const c = store.credentials.find(x => x.id === selectedId.value)
  if (!c) return
  const cfg = { ...(props.node.config ?? {}) }
  cfg.credentialId = c.id
  cfg.credentialName = c.name
  store.updateNodeConfig(props.node.id, cfg)
  emit('update:open', false)
}

// ── 新建凭据 ──
const creating = ref(false)
const KIND_OPTIONS = [
  { label: '用户名+密码', value: 'basic' },
  { label: 'Personal Access Token', value: 'pat' },
  { label: 'API Token', value: 'token' },
  { label: 'SSH 私钥', value: 'ssh-key' },
  { label: 'Kubeconfig', value: 'kubeconfig' },
]
const PLATFORM_OPTIONS = [
  { label: 'Docker', value: 'docker' },
  { label: 'GitLab', value: 'gitlab' },
  { label: 'Jenkins', value: 'jenkins' },
  { label: 'Harbor', value: 'harbor' },
  { label: 'K8S', value: 'k8s' },
  { label: 'SSH', value: 'ssh' },
]
const newCred = ref({ name: '', kind: 'basic' as CredentialKind, platform: 'docker', username: '', password: '', token: '', url: '' })

watch(() => props.open, (v) => { if (v) { creating.value = false; selectedId.value = '' } })

async function saveNew() {
  if (!props.node || !newCred.value.name.trim()) return
  const fields: Record<string, string> = {}
  if (newCred.value.username) fields.username = newCred.value.username
  if (newCred.value.password) fields.password = newCred.value.password
  if (newCred.value.token) fields.token = newCred.value.token
  if (newCred.value.url) fields.url = newCred.value.url
  const meta = await store.credentialSave({
    name: newCred.value.name.trim(),
    kind: newCred.value.kind,
    platform: newCred.value.platform,
    fields,
  })
  const cfg = { ...(props.node.config ?? {}) }
  cfg.credentialId = meta.id
  cfg.credentialName = meta.name
  store.updateNodeConfig(props.node.id, cfg)
  emit('update:open', false)
}

function unbind() {
  if (!props.node) return
  const cfg = { ...(props.node.config ?? {}) }
  delete cfg.credentialId
  delete cfg.credentialName
  store.updateNodeConfig(props.node.id, cfg)
  emit('update:open', false)
}

const showSecret = computed(() => newCred.value.kind === 'basic' || newCred.value.kind === 'ssh-key')
</script>

<template>
  <JcModal :open="open" title="配置凭据" width="440" :footer="false" @update:open="emit('update:open', $event)">
    <div class="lgd">
      <template v-if="bound">
        <div class="lgd-bound">
          <div class="lgd-bound-title">已绑定凭据</div>
          <div class="lgd-bound-row">
            <span class="lgd-key">{{ bound.credentialName }}</span>
            <span class="lgd-meta">通过「凭据端口」连线引用到目标积木</span>
          </div>
          <div class="lgd-actions">
            <JcButton size="small" danger @click="unbind">解除绑定</JcButton>
            <JcButton size="small" @click="creating = true">重新绑定</JcButton>
          </div>
        </div>
      </template>

      <template v-if="!bound || creating">
        <!-- 选择已有凭据 -->
        <div class="lgd-sec">
          <div class="lgd-sec-title">选择已有凭据</div>
          <div class="lgd-row">
            <JcSelect
              :model-value="selectedId"
              :options="credentialOptions"
              placeholder="选择凭据（仅显示掩码）"
              @update:model-value="(v) => (selectedId = String(v))"
            />
            <JcButton size="small" type="primary" :disabled="!selectedId" @click="bindExisting">绑定</JcButton>
          </div>
          <div v-if="store.credentials.length === 0" class="lgd-tip">还没有凭据，可新建一个</div>
        </div>

        <!-- 新建凭据 -->
        <div class="lgd-sec">
          <div class="lgd-sec-title">新建凭据</div>
          <div class="lgd-grid">
            <JcInput v-model="newCred.name" placeholder="凭据名称（如 生产 GitLab）" />
            <JcSelect
              :model-value="newCred.kind"
              :options="KIND_OPTIONS"
              @update:model-value="(v) => (newCred.kind = v as CredentialKind)"
            />
            <JcSelect
              :model-value="newCred.platform"
              :options="PLATFORM_OPTIONS"
              @update:model-value="(v) => (newCred.platform = String(v))"
            />
            <JcInput v-if="showSecret" v-model="newCred.username" placeholder="用户名" />
            <JcInput
              v-if="newCred.kind === 'basic'"
              v-model="newCred.password"
              type="password"
              placeholder="密码"
            />
            <JcInput
              v-else
              v-model="newCred.token"
              type="password"
              :placeholder="newCred.kind === 'kubeconfig' ? 'Kubeconfig 内容' : 'Token'"
            />
            <JcInput v-model="newCred.url" placeholder="平台地址（可选）" />
          </div>
          <div class="lgd-actions">
            <JcButton size="small" type="primary" :disabled="!newCred.name.trim()" @click="saveNew">保存并绑定</JcButton>
          </div>
        </div>
      </template>
    </div>
  </JcModal>
</template>

<style scoped lang="scss">
.lgd {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.lgd-bound {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.lgd-bound-title,
.lgd-sec-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-primary, #e6e6e6);
}
.lgd-bound-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border: 1px solid var(--jc-color-accent, #8a58ff);
  border-radius: 6px;
  background: color-mix(in srgb, var(--jc-color-accent, #8a58ff) 12%, transparent);
}
.lgd-key {
  font-size: 13px;
  color: var(--jc-text-primary, #e6e6e6);
}
.lgd-meta {
  font-size: 11px;
  color: var(--jc-text-secondary, #aaa);
}
.lgd-sec {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.lgd-row {
  display: flex;
  gap: 8px;
  align-items: center;
}
.lgd-row > :first-child {
  flex: 1;
}
.lgd-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.lgd-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
}
.lgd-tip {
  font-size: 11px;
  color: var(--jc-text-tertiary, #858585);
}
</style>
