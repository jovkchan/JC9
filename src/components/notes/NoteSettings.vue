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

// ── AI 助理配置状态 ──
const aiProvider = ref<'ollama' | 'deepseek' | 'openai' | 'gemini' | 'vllm'>('ollama')
const aiEndpoint = ref('http://127.0.0.1:11434')
const aiApiKey = ref('')
const aiModel = ref('llama3')

const vllmModels = ref<string[]>([])
const loadingModels = ref(false)

async function fetchVllmModels() {
  if (aiProvider.value !== 'vllm') return
  loadingModels.value = true
  try {
    const res = await fetch(`${aiEndpoint.value}/models`)
    if (res.ok) {
      const json = await res.json()
      if (json.data && Array.isArray(json.data)) {
        vllmModels.value = json.data.map((m: any) => m.id)
        if (vllmModels.value.length > 0 && !vllmModels.value.includes(aiModel.value)) {
          aiModel.value = vllmModels.value[0]
        }
      }
    }
  } catch (e) {
    console.error('vLLM 拉取失败:', e)
  } finally {
    loadingModels.value = false
  }
}

onMounted(() => {
  // 读取偏好
  defaultFormat.value = (localStorage.getItem('notes-default-format') as any) || 'markdown'
  defaultVisibility.value = (localStorage.getItem('notes-default-visibility') as any) || 'PRIVATE'

  // 读取 AI 配置
  aiProvider.value = (localStorage.getItem('notes-ai-provider') as any) || 'ollama'
  aiEndpoint.value = localStorage.getItem('notes-ai-endpoint') || 'http://127.0.0.1:11434'
  aiApiKey.value = localStorage.getItem('notes-ai-apikey') || ''
  aiModel.value = localStorage.getItem('notes-ai-model') || 'llama3'
  if (aiProvider.value === 'vllm') {
    fetchVllmModels()
  }
})

function saveSettings() {
  localStorage.setItem('notes-default-format', defaultFormat.value)
  localStorage.setItem('notes-default-visibility', defaultVisibility.value)
  localStorage.setItem('notes-ai-provider', aiProvider.value)
  localStorage.setItem('notes-ai-endpoint', aiEndpoint.value)
  localStorage.setItem('notes-ai-apikey', aiApiKey.value)
  localStorage.setItem('notes-ai-model', aiModel.value)
  
  // 独立保存各供应商的配置
  localStorage.setItem(`notes-ai-endpoint-${aiProvider.value}`, aiEndpoint.value)
  localStorage.setItem(`notes-ai-apikey-${aiProvider.value}`, aiApiKey.value)
  localStorage.setItem(`notes-ai-model-${aiProvider.value}`, aiModel.value)

  status.pushMessage('设置保存成功', 'success')
  emit('close')
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

watchProvider()
function watchProvider() {
  const savedEndpoint = localStorage.getItem(`notes-ai-endpoint-${aiProvider.value}`)
  const savedApiKey = localStorage.getItem(`notes-ai-apikey-${aiProvider.value}`)
  const savedModel = localStorage.getItem(`notes-ai-model-${aiProvider.value}`)

  if (savedEndpoint) aiEndpoint.value = savedEndpoint
  if (savedApiKey) aiApiKey.value = savedApiKey
  if (savedModel) aiModel.value = savedModel

  // 若没有已存参数，则加载出厂预设默认值
  if (!savedEndpoint || !savedModel) {
    if (aiProvider.value === 'ollama') {
      aiEndpoint.value = 'http://127.0.0.1:11434'
      aiModel.value = 'llama3'
    } else if (aiProvider.value === 'deepseek') {
      aiEndpoint.value = 'https://api.deepseek.com/v1'
      aiModel.value = 'deepseek-chat'
    } else if (aiProvider.value === 'openai') {
      aiEndpoint.value = 'https://api.openai.com/v1'
      aiModel.value = 'gpt-4o-mini'
    } else if (aiProvider.value === 'gemini') {
      aiEndpoint.value = 'https://generativelanguage.googleapis.com'
      aiModel.value = 'gemini-1.5-flash'
    } else if (aiProvider.value === 'vllm') {
      aiEndpoint.value = 'http://192.168.5.100:8000/v1'
      aiModel.value = ''
    }
  }

  if (aiProvider.value === 'vllm') {
    fetchVllmModels()
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
            <h3 class="pane-title">AI Copilot 模型连接</h3>
            <div class="form-group">
              <label>大模型供应商</label>
              <select v-model="aiProvider" @change="watchProvider" class="form-select">
                <option value="ollama">Ollama (本地离线，免Key)</option>
                <option value="deepseek">DeepSeek (高性能性价比)</option>
                <option value="openai">OpenAI (ChatGPT)</option>
                <option value="gemini">Google Gemini</option>
                <option value="vllm">vLLM (本地部署)</option>
              </select>
            </div>

            <div class="form-group">
              <label>API Endpoint (接口地址)</label>
              <input v-model="aiEndpoint" class="form-input" placeholder="输入 Endpoint 网址..." />
            </div>

            <div class="form-group" v-if="aiProvider !== 'ollama' && aiProvider !== 'vllm'">
              <label>API Key (密钥)</label>
              <input v-model="aiApiKey" type="password" class="form-input" placeholder="输入 API 密钥..." />
            </div>

            <div class="form-group">
              <label>Model Name (模型代号)</label>
              <select v-if="aiProvider === 'vllm'" v-model="aiModel" class="form-select">
                <option v-if="loadingModels" value="">正在获取模型列表...</option>
                <option v-else-if="vllmModels.length === 0" value="">暂无可用模型 (连接失败)</option>
                <option v-for="m in vllmModels" :key="m" :value="m">{{ m }}</option>
              </select>
              <input v-else v-model="aiModel" class="form-input" placeholder="例如 llama3 或 gpt-4o-mini..." />
            </div>
          </div>

          <!-- 3. 数据备份导入 -->
          <div v-if="activeTab === 'backup'" class="settings-pane">
            <h3 class="pane-title">数据本地备份与导入恢复</h3>
            <p class="pane-desc">因为所有备忘均保存在本地 SQLite 数据库中，您可以导出 JSON 数据包保存至本地，也可以通过 JSON 备份包将所有记录恢复至本软件中。</p>

            <div class="backup-actions">
              <button class="backup-btn export" @click="exportData">
                📤 备份并导出 JSON
              </button>
              <button class="backup-btn import" @click="importData">
                📥 导入并恢复 JSON
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
</style>
