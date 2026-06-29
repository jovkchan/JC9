<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useNotesStore } from '@/stores/notes'
import { useStatusStore } from '@/stores/status'
import { save, open } from '@tauri-apps/plugin-dialog'
import { invoke } from '@tauri-apps/api/core'

defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  close: []
}>()

const store = useNotesStore()
const status = useStatusStore()

const activeTab = ref<'general' | 'ai' | 'backup'>('general')

// ── 设置偏好状态 (存入 localStorage) ──
const defaultFormat = ref<'markdown' | 'plain'>('markdown')
const defaultVisibility = ref<'PRIVATE' | 'PUBLIC'>('PRIVATE')

// ── AI 模型配置列表 ──
interface ModelConfig {
  id: string
  name: string
  provider: 'ollama' | 'deepseek' | 'openai' | 'gemini' | 'vllm'
  endpoint: string
  apiKey: string
  model: string
  inputPrice: number
  outputPrice: number
  costLimit: number
  reasoningEffort: 'high' | 'max' | ''  // DS thinking mode 强度，空=关闭
}

const modelConfigs = ref<ModelConfig[]>([])
const editingModel = ref<ModelConfig | null>(null)
const showModelForm = ref(false)
const vllmModels = ref<string[]>([])
const loadingModels = ref(false)

function blankModel(): ModelConfig {
  return {
    id: '',
    name: '',
    provider: 'deepseek',
    endpoint: 'https://api.deepseek.com',
    apiKey: '',
    model: 'deepseek-v4-pro',
    inputPrice: 2.0,
    outputPrice: 4.0,
    costLimit: 10.0,
    reasoningEffort: 'high',
  }
}

const newModelForm = ref<ModelConfig>(blankModel())

// ── 备份与导入 ──

onMounted(() => {
  defaultFormat.value = (localStorage.getItem('notes-default-format') as any) || 'markdown'
  defaultVisibility.value = (localStorage.getItem('notes-default-visibility') as any) || 'PRIVATE'

  // 读取模型配置列表
  const saved = localStorage.getItem('notes-ai-models')
  if (saved) {
    try { modelConfigs.value = JSON.parse(saved) } catch { /* ignore */ }
  }
  // 兼容旧版单模型配置
  if (modelConfigs.value.length === 0) {
    const legacy: ModelConfig = {
      id: 'legacy',
      name: '默认配置',
      provider: (localStorage.getItem('notes-ai-provider') as any) || 'deepseek',
      endpoint: localStorage.getItem('notes-ai-endpoint') || 'https://api.deepseek.com',
      apiKey: localStorage.getItem('notes-ai-apikey') || '',
      model: localStorage.getItem('notes-ai-model') || 'deepseek-v4-pro',
      inputPrice: parseFloat(localStorage.getItem('notes-ai-input-price') || '3.0'),
      outputPrice: parseFloat(localStorage.getItem('notes-ai-output-price') || '6.0'),
      costLimit: parseFloat(localStorage.getItem('notes-ai-cost-limit') || '5.0'),
    }
    modelConfigs.value = [legacy]
  }
})

function saveSettings() {
  localStorage.setItem('notes-default-format', defaultFormat.value)
  localStorage.setItem('notes-default-visibility', defaultVisibility.value)
  localStorage.setItem('notes-ai-models', JSON.stringify(modelConfigs.value))

  // 兼容旧字段
  const first = modelConfigs.value[0]
  if (first) {
    localStorage.setItem('notes-ai-provider', first.provider)
    localStorage.setItem('notes-ai-endpoint', first.endpoint)
    localStorage.setItem('notes-ai-apikey', first.apiKey)
    localStorage.setItem('notes-ai-model', first.model)
    localStorage.setItem(`notes-ai-endpoint-${first.provider}`, first.endpoint)
    localStorage.setItem(`notes-ai-apikey-${first.provider}`, first.apiKey)
    localStorage.setItem(`notes-ai-model-${first.provider}`, first.model)

    // 同步 Agent cost config
    import('@/stores/ai').then(({ useAiStore }) => {
      useAiStore().updateCostConfig({
        inputCachedCostPerM: first.inputPrice * 0.008,
        inputUncachedCostPerM: first.inputPrice,
        outputCostPerM: first.outputPrice,
        costLimit: first.costLimit,
      })
    })
  }
  status.pushMessage('设置保存成功', 'success')
  emit('close')
}

function addModel() {
  newModelForm.value = blankModel()
  showModelForm.value = true
}

function editModel(config: ModelConfig) {
  newModelForm.value = { ...config }
  showModelForm.value = true
}

function deleteModel(id: string) {
  modelConfigs.value = modelConfigs.value.filter(c => c.id !== id)
}

function saveModelForm() {
  const f = newModelForm.value
  if (!f.name.trim() || !f.model.trim()) {
    status.pushMessage('请填写模型名称和代号', 'warn')
    return
  }
  if (!f.id) f.id = Date.now().toString()
  const idx = modelConfigs.value.findIndex(c => c.id === f.id)
  if (idx >= 0) {
    modelConfigs.value[idx] = { ...f }
  } else {
    modelConfigs.value.push({ ...f })
  }
  showModelForm.value = false
}

function cancelModelForm() {
  showModelForm.value = false
}

function setProviderDefaults() {
  const f = newModelForm.value
  if (f.provider === 'ollama') { f.endpoint = 'http://127.0.0.1:11434'; f.model = 'llama3' }
  else if (f.provider === 'deepseek') { f.endpoint = 'https://api.deepseek.com'; f.model = 'deepseek-v4-pro' }
  else if (f.provider === 'openai') { f.endpoint = 'https://api.openai.com/v1'; f.model = 'gpt-4o-mini' }
  else if (f.provider === 'gemini') { f.endpoint = 'https://generativelanguage.googleapis.com'; f.model = 'gemini-1.5-flash' }
  else if (f.provider === 'vllm') { f.endpoint = 'http://192.168.5.100:8000/v1'; f.model = ''; fetchVllmModelsForm() }
}

async function fetchVllmModelsForm() {
  loadingModels.value = true
  try {
    const res = await fetch(`${newModelForm.value.endpoint}/models`)
    if (res.ok) {
      const json = await res.json()
      if (json.data && Array.isArray(json.data)) {
        vllmModels.value = json.data.map((m: any) => m.id)
      }
    }
  } catch { /* ignore */ }
  finally { loadingModels.value = false }
}

// ── 备份与导出 ──
async function exportData() {
  try {
    const filePath = await save({
      filters: [{ name: 'JSON Backup', extensions: ['json'] }],
      defaultPath: 'jc9-memos-backup.json'
    })

    if (filePath) {
      const dataStr = JSON.stringify({
        notes: store.notes,
        groups: store.groups
      }, null, 2)

      const encoder = new TextEncoder()
      const binaryData = Array.from(encoder.encode(dataStr))

      await invoke('write_file_binary', { path: filePath, data: binaryData })
      status.pushMessage('备份文件导出成功！', 'success')
    }
  } catch (e) {
    status.pushMessage(`导出失败: ${e}`, 'error')
  }
}

// ── 备份与导入 ──
async function importData() {
  try {
    const selected = await open({
      filters: [{ name: 'JSON Backup', extensions: ['json'] }],
      multiple: false
    })

    if (selected && typeof selected === 'string') {
      // 引入 Rust 端读取文本文件的接口
      const content = await invoke<string>('read_file_string', { path: selected })
      const data = JSON.parse(content)

      if (!data.notes || !Array.isArray(data.notes)) {
        status.pushMessage('无效的备份文件结构', 'error')
        return
      }

      // 批量覆盖式保存到本地数据库
      for (const note of data.notes) {
        await store.saveNote(note)
      }
      if (data.groups && Array.isArray(data.groups)) {
        for (const g of data.groups) {
          await store.updateGroup(g)
        }
      }

      await store.loadAllNotes()
      await store.loadGroups()
      status.pushMessage('备份数据导入恢复成功！', 'success')
    }
  } catch (e) {
    status.pushMessage(`导入失败: ${e}`, 'error')
  }
}

</script>

<template>
  <div v-if="show" class="settings-overlay" @click.self="emit('close')">
    <div class="settings-modal animate-slide-in">
      <div class="settings-header">
        <span class="settings-title">备忘笔记设置</span>
        <button class="settings-close-btn" @click="emit('close')">✕</button>
      </div>

      <div class="settings-container">
        <!-- 侧边 Tab 导航 -->
        <aside class="settings-nav">
          <div :class="['nav-item', { active: activeTab === 'general' }]" @click="activeTab = 'general'">
            通用设置
          </div>
          <div :class="['nav-item', { active: activeTab === 'ai' }]" @click="activeTab = 'ai'">
            AI 助理配置
          </div>
          <div :class="['nav-item', { active: activeTab === 'backup' }]" @click="activeTab = 'backup'">
            数据备份导入
          </div>
        </aside>

        <!-- 主内容区 -->
        <main class="settings-content">
          <!-- 1. 通用设置 -->
          <div v-if="activeTab === 'general'" class="settings-pane">
            <h3 class="pane-title">偏好设置</h3>
            <div class="form-group">
              <label>默认笔记格式</label>
              <select v-model="defaultFormat" class="form-select">
                <option value="markdown">Markdown (推荐)</option>
                <option value="plain">纯文本</option>
              </select>
              <span class="help-text">新建备忘时的默认输入解析格式</span>
            </div>

            <div class="form-group">
              <label>新建笔记默认可见性</label>
              <select v-model="defaultVisibility" class="form-select">
                <option value="PRIVATE">PRIVATE (私有本地)</option>
                <option value="PUBLIC">PUBLIC (公开/对接远端后可见)</option>
              </select>
              <span class="help-text">第一期完全本地化下默认均为 PRIVATE 级别</span>
            </div>
          </div>

          <!-- 2. AI 助理配置 -->
          <div v-if="activeTab === 'ai'" class="settings-pane">
            <h3 class="pane-title">AI 模型配置</h3>
            <p class="pane-desc" style="margin-bottom:12px">管理您接入的大模型供应商，每个模型独立配置计费与熔断限额。</p>

            <!-- 模型列表 -->
            <div class="model-list">
              <div v-for="cfg in modelConfigs" :key="cfg.id" class="model-card">
                <div class="model-card-header">
                  <span class="model-card-name">{{ cfg.name }}</span>
                  <span class="model-card-provider">{{ cfg.provider }}</span>
                  <span class="model-card-model">{{ cfg.model }}</span>
                </div>
                <!-- <div class="model-card-meta">
                  <span v-if="cfg.reasoningEffort">🧠 {{ cfg.reasoningEffort }}</span>
                  <span>输入 ¥{{ cfg.inputPrice }}/M</span>
                  <span>输出 ¥{{ cfg.outputPrice }}/M</span>
                  <span>限额 ¥{{ cfg.costLimit }}</span>
                </div> -->
                <div class="model-card-actions">
                  <button class="model-btn edit" @click="editModel(cfg)">编辑</button>
                  <button class="model-btn del" @click="deleteModel(cfg.id)">删除</button>
                </div>
              </div>
              <div v-if="modelConfigs.length === 0" class="empty-hint">尚未添加任何模型</div>
            </div>

            <button class="add-model-btn" @click="addModel">+ 添加模型</button>

            <!-- 添加/编辑模型表单 -->
            <div v-if="showModelForm" class="model-form-overlay" @click.self="cancelModelForm">
              <div class="model-form-card">
                <h4>{{ newModelForm.id ? '编辑' : '添加' }}模型配置</h4>
                <div class="form-group">
                  <label>配置名称</label>
                  <input v-model="newModelForm.name" class="form-input" placeholder="例如：DeepSeek 主力" />
                </div>
                <div class="form-group">
                  <label>供应商</label>
                  <select v-model="newModelForm.provider" @change="setProviderDefaults" class="form-select">
                    <option value="deepseek">DeepSeek</option>
                    <option value="openai">OpenAI</option>
                    <option value="ollama">Ollama (本地)</option>
                    <option value="gemini">Google Gemini</option>
                    <option value="vllm">vLLM (自部署)</option>
                  </select>
                </div>
                <div class="form-group">
                  <label>Endpoint</label>
                  <input v-model="newModelForm.endpoint" class="form-input" />
                </div>
                <div class="form-group" v-if="newModelForm.provider !== 'ollama' && newModelForm.provider !== 'vllm'">
                  <label>API Key</label>
                  <input v-model="newModelForm.apiKey" type="password" class="form-input" placeholder="sk-..." />
                </div>
                <div class="form-group">
                  <label>Model</label>
                  <select v-if="newModelForm.provider === 'vllm'" v-model="newModelForm.model" class="form-select">
                    <option v-if="loadingModels" value="">获取中...</option>
                    <option v-for="m in vllmModels" :key="m" :value="m">{{ m }}</option>
                  </select>
                  <input v-else v-model="newModelForm.model" class="form-input" placeholder="多个用英文逗号分隔，如: gemini-1.5-flash, gemini-1.5-pro" />
                  <span class="help-text" v-if="newModelForm.provider !== 'vllm'">支持输入多个模型，请使用英文逗号 <code>,</code> 分隔。</span>
                </div>
                <div class="form-group" v-if="newModelForm.provider === 'deepseek'">
                  <label>思维强度 (Thinking Mode)</label>
                  <select v-model="newModelForm.reasoningEffort" class="form-select">
                    <option value="high">High (推荐，大多数场景)</option>
                    <option value="max">Max (复杂 Agent 任务)</option>
                    <option value="">关闭 (节省 Token)</option>
                  </select>
                  <span class="help-text">DeepSeek 思考模式：high 适合日常编码，max 适合复杂多步推理</span>
                </div>
                <div class="form-row">
                  <div class="form-group form-half">
                    <label>输入价格 (元/百万)</label>
                    <input v-model.number="newModelForm.inputPrice" type="number" step="0.1" class="form-input" />
                  </div>
                  <div class="form-group form-half">
                    <label>输出价格 (元/百万)</label>
                    <input v-model.number="newModelForm.outputPrice" type="number" step="0.1" class="form-input" />
                  </div>
                </div>
                <div class="form-group">
                  <label>熔断限额 (元)</label>
                  <input v-model.number="newModelForm.costLimit" type="number" step="0.5" class="form-input" />
                </div>
                <div class="model-form-actions">
                  <button class="footer-btn-cancel" @click="cancelModelForm">取消</button>
                  <button class="footer-btn-save" @click="saveModelForm">确定</button>
                </div>
              </div>
            </div>
          </div>

          <!-- 3. 数据备份导入 -->
          <div v-if="activeTab === 'backup'" class="settings-pane">
            <h3 class="pane-title">数据本地备份与导入恢复</h3>
            <p class="pane-desc">因为所有备忘均保存在本地 SQLite 数据库中，您可以导出 JSON 数据包保存至本地，也可以通过 JSON 备份包将所有记录恢复至本软件中。</p>

            <div class="backup-actions">
              <button class="backup-btn export" @click="exportData">
                备份并导出 JSON
              </button>
              <button class="backup-btn import" @click="importData">
                导入并恢复 JSON
              </button>
            </div>
          </div>
        </main>
      </div>

      <div class="settings-footer">
        <button class="footer-btn-cancel" @click="emit('close')">取消</button>
        <button class="footer-btn-save" @click="saveSettings">保存配置</button>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.settings-overlay {
  position: fixed;
  inset: 0;
  background: var(--jc-bg-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1200;
}

.settings-modal {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  width: 580px;
  max-width: 90%;
  height: 400px;
  display: flex;
  flex-direction: column;
  box-shadow: var(--jc-shadow-modal);
  border-radius: 8px;
  overflow: hidden;
}

.settings-header {
  background: var(--jc-bg-panel);
  padding: 12px 16px;
  display: flex;
  justify-content: space-between;
  align-items: center;
  border-bottom: 1px solid var(--jc-border-default);

  .settings-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--jc-text-highlight);
  }

  .settings-close-btn {
    background: none;
    border: none;
    color: var(--jc-text-secondary);
    font-size: 14px;
    cursor: pointer;

    &:hover {
      color: var(--jc-color-error);
    }
  }
}

.settings-container {
  display: flex;
  flex: 1;
  min-height: 0;
}

.settings-nav {
  width: 160px;
  background: var(--jc-bg-panel);
  border-right: 1px solid var(--jc-border-default);
  padding: 12px 0;
  display: flex;
  flex-direction: column;
  gap: 2px;

  .nav-item {
    padding: 8px 16px;
    font-size: 12px;
    color: var(--jc-text-secondary);
    cursor: pointer;
    transition: background 0.15s, color 0.15s;

    &:hover {
      background: var(--jc-bg-hover);
      color: var(--jc-text-primary);
    }

    &.active {
      background: var(--jc-bg-selected);
      color: var(--jc-color-accent);
      font-weight: 600;
    }
  }
}

.settings-content {
  flex: 1;
  padding: 16px;
  overflow-y: auto;
}

.settings-pane {
  display: flex;
  flex-direction: column;
  gap: 12px;

  .pane-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--jc-text-highlight);
    margin-bottom: 4px;
    border-bottom: 1px solid var(--jc-border-default);
    padding-bottom: 6px;
  }

  .pane-desc {
    font-size: 11px;
    color: var(--jc-text-secondary);
    line-height: 1.6;
  }
}

.form-group {
  display: flex;
  flex-direction: column;
  gap: 4px;

  label {
    font-size: 11px;
    font-weight: 500;
    color: var(--jc-text-primary);
  }

  .form-select,
  .form-input {
    background: var(--jc-bg-input);
    border: 1px solid var(--jc-border-default);
    color: var(--jc-text-primary);
    font-size: 12px;
    padding: 6px 10px;
    border-radius: 4px;
    outline: none;

    &:focus {
      border-color: var(--jc-color-accent);
    }
  }

  .help-text {
    font-size: 10px;
    color: var(--jc-text-secondary);
    opacity: 0.8;
  }
}

.backup-actions {
  display: flex;
  gap: 12px;
  margin-top: 16px;
}

.backup-btn {
  flex: 1;
  padding: 10px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: opacity 0.2s;

  &.export {
    background: rgba(var(--jc-color-accent-rgb, 0, 102, 204), 0.1);
    color: var(--jc-color-accent);
    border: 1px solid var(--jc-color-accent);
  }

  &.import {
    background: rgba(var(--jc-color-success-rgb, 0, 109, 50), 0.1);
    color: var(--jc-color-success);
    border: 1px solid var(--jc-color-success);
  }

  &:hover {
    opacity: 0.9;
  }
}

.settings-footer {
  padding: 12px 16px;
  background: var(--jc-bg-panel);
  border-top: 1px solid var(--jc-border-default);
  display: flex;
  justify-content: flex-end;
  gap: 8px;

  .footer-btn-cancel {
    background: var(--jc-bg-btn);
    color: var(--jc-text-secondary);
    border: none;
    padding: 6px 14px;
    font-size: 12px;
    border-radius: 4px;
    cursor: pointer;

    &:hover {
      color: var(--jc-text-primary);
    }
  }

  .footer-btn-save {
    background: var(--jc-color-accent);
    color: #fff;
    border: none;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 600;
    border-radius: 4px;
    cursor: pointer;

    &:hover {
      opacity: 0.9;
    }
  }
}

// 动画
.animate-slide-in {
  animation: slideIn 0.2s ease-out;
}

@keyframes slideIn {
  from {
    opacity: 0;
    transform: translateY(20px);
  }

  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* ── 模型列表 ── */
.model-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-height: 240px;
  overflow-y: auto;
  margin-bottom: 10px;
}

.model-card {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--jc-border-default);
  border-radius: 6px;
  font-size: 12px;
}

.model-card-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
}

.model-card-name {
  font-weight: 600;
  color: var(--jc-text-primary);
}

.model-card-provider {
  font-size: 10px;
  padding: 1px 6px;
  border-radius: 4px;
  background: rgba(255, 255, 255, 0.06);
  color: var(--jc-text-secondary);
}

.model-card-model {
  font-family: monospace;
  font-size: 11px;
  color: #58a6ff;
}

.model-card-meta {
  display: flex;
  gap: 10px;
  font-size: 10px;
  color: var(--jc-text-secondary);
  margin: 0 12px;
  white-space: nowrap;
}

.model-card-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.model-btn {
  padding: 2px 8px;
  border: 1px solid var(--jc-border-default);
  border-radius: 4px;
  font-size: 10px;
  cursor: pointer;
  background: var(--jc-bg-elevated);
  color: var(--jc-text-primary);

  &.edit:hover {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }

  &.del:hover {
    border-color: #f85149;
    color: #f85149;
  }
}

.add-model-btn {
  width: 100%;
  padding: 6px;
  border: 1px dashed var(--jc-border-default);
  border-radius: 6px;
  background: transparent;
  color: var(--jc-text-secondary);
  font-size: 12px;
  cursor: pointer;

  &:hover {
    border-color: var(--jc-color-accent);
    color: var(--jc-color-accent);
  }
}

.empty-hint {
  text-align: center;
  color: var(--jc-text-secondary);
  font-size: 12px;
  padding: 16px;
}

/* ── 模型表单弹窗 ── */
.model-form-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}

.model-form-card {
  background: var(--jc-bg-elevated);
  border: 1px solid var(--jc-border-strong);
  border-radius: 8px;
  padding: 16px;
  width: 360px;
  max-height: 90%;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.4);

  h4 {
    margin: 0;
    font-size: 13px;
    color: var(--jc-text-primary);
  }
}

.model-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}

.pane-subtitle {
  font-size: 11px;
  font-weight: 600;
  color: #f0883e;
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid var(--jc-border-default);
}

.form-row {
  display: flex;
  gap: 8px;
}

.form-half {
  flex: 1;
}
</style>
