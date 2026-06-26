<script setup lang="ts">
import { ref } from 'vue'

const domain = ref('github.com')
const recordType = ref('A')
const dohProvider = ref<'cloudflare' | 'alidns'>('cloudflare')
const loading = ref(false)
const errorMsg = ref('')

interface DnsRecord {
  name: string
  type: number
  TTL: number
  data: string
}

interface DnsResponse {
  Status: number
  TC: boolean
  RD: boolean
  RA: boolean
  AD: boolean
  CD: boolean
  Question: { name: string; type: number }[]
  Answer?: DnsRecord[]
}

const statusText = ref('')
const questionList = ref<{ name: string; type: string }[]>([])
const answerList = ref<{ name: string; type: string; ttl: number; data: string }[]>([])
const rawJson = ref('')

const DNS_TYPES: Record<number, string> = {
  1: 'A',
  2: 'NS',
  5: 'CNAME',
  6: 'SOA',
  15: 'MX',
  16: 'TXT',
  28: 'AAAA'
}

const STATUS_MAP: Record<number, string> = {
  0: 'NOERROR (解析成功)',
  1: 'FORMERR (格式错误)',
  2: 'SERVFAIL (服务器失败)',
  3: 'NXDOMAIN (域名不存在)',
  4: 'NOTIMP (未实现该查询)',
  5: 'REFUSED (查询被拒绝)'
}

async function resolveDns() {
  const queryDomain = domain.value.trim()
  if (!queryDomain) {
    errorMsg.value = '请输入有效的域名！'
    return
  }

  loading.value = true
  errorMsg.value = ''
  rawJson.value = ''
  answerList.value = []
  questionList.value = []
  statusText.value = ''

  let url = ''
  if (dohProvider.value === 'cloudflare') {
    url = `https://cloudflare-dns.com/dns-query?name=${encodeURIComponent(queryDomain)}&type=${recordType.value}`
  } else {
    // 阿里云 DoH 接口
    url = `https://dns.alicdn.com/resolve?name=${encodeURIComponent(queryDomain)}&type=${recordType.value}`
  }

  try {
    const res = await fetch(url, {
      method: 'GET',
      headers: {
        'accept': 'application/dns-json'
      }
    })

    if (!res.ok) {
      throw new Error(`HTTP 异常，状态码: ${res.status}`)
    }

    const data: DnsResponse = await res.json()
    rawJson.value = JSON.stringify(data, null, 2)

    statusText.value = STATUS_MAP[data.Status] || `UNKNOWN (${data.Status})`

    if (data.Question) {
      questionList.value = data.Question.map(q => ({
        name: q.name,
        type: DNS_TYPES[q.type] || String(q.type)
      }))
    }

    if (data.Answer && data.Answer.length > 0) {
      answerList.value = data.Answer.map(a => ({
        name: a.name,
        type: DNS_TYPES[a.type] || String(a.type),
        ttl: a.TTL,
        data: a.data
      }))
    } else {
      answerList.value = []
    }
  } catch (err: any) {
    errorMsg.value = `查询失败: ${err.message || '网络连接异常，请检查 DNS 节点是否可用。'}`
  } finally {
    loading.value = false
  }
}

function copyJson() {
  if (!rawJson.value) return
  navigator.clipboard.writeText(rawJson.value)
}

function copyAnswersText() {
  if (answerList.value.length === 0) return
  const text = answerList.value.map(a => `${a.name}\t${a.ttl}\tIN\t${a.type}\t${a.data}`).join('\n')
  navigator.clipboard.writeText(text)
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">DNS 解析查询 (dig)</div>
    </div>

    <div class="tool-body-split">
      <!-- 左侧输入栏 -->
      <div class="control-panel">
        <div class="setting-section">
          <div class="section-subtitle">域名及类型设置</div>
          <div class="input-row">
            <input 
              v-model="domain" 
              placeholder="请输入域名，如 github.com" 
              class="domain-input" 
              @keyup.enter="resolveDns"
            />
            <select v-model="recordType" class="type-select">
              <option value="A">A 记录</option>
              <option value="CNAME">CNAME 记录</option>
              <option value="AAAA">AAAA 记录</option>
              <option value="MX">MX 记录</option>
              <option value="TXT">TXT 记录</option>
              <option value="NS">NS 记录</option>
            </select>
          </div>
        </div>

        <div class="setting-section">
          <div class="section-subtitle">DNS-over-HTTPS (DoH) 线路选择</div>
          <div class="doh-choice-row">
            <label class="radio-label">
              <input type="radio" value="cloudflare" v-model="dohProvider" />
              <span>Cloudflare (全球最佳)</span>
            </label>
            <label class="radio-label">
              <input type="radio" value="alidns" v-model="dohProvider" />
              <span>AliDNS 阿里 (国内直连)</span>
            </label>
          </div>
        </div>

        <button 
          class="tool-btn pri full large" 
          :disabled="loading" 
          @click="resolveDns"
        >
          {{ loading ? '正在进行 DNS 查询...' : '域名解析查询 (dig)' }}
        </button>

        <div v-if="errorMsg" class="tool-footer-error style-inline mt-10">{{ errorMsg }}</div>
      </div>

      <!-- 右侧结果展示栏 (模拟终端 dig 输出) -->
      <div class="result-display-panel">
        <div class="pane-header-row">
          <span>终端 dig 输出模拟</span>
          <div class="acts">
            <button class="tool-btn small" :disabled="!rawJson" @click="copyAnswersText">复制记录</button>
            <button class="tool-btn small" :disabled="!rawJson" @click="copyJson">复制 JSON</button>
          </div>
        </div>

        <div class="terminal-view">
          <div class="terminal-line comment">; &lt;&lt;&gt;&gt; DiG-over-HTTPS 1.0 &lt;&lt;&gt;&gt; {{ domain }} {{ recordType }}</div>
          <div class="terminal-line comment">; (Using DoH Server: {{ dohProvider === 'cloudflare' ? 'https://cloudflare-dns.com' : 'https://dns.alicdn.com' }})</div>
          
          <template v-if="statusText">
            <div class="terminal-line mt-10">;; ->>HEADER<<- opcode: QUERY, status: <span class="status-val" :class="{ error: statusText.includes('FAIL') || statusText.includes('ERR') }">{{ statusText }}</span></div>
            
            <div class="terminal-line mt-10 comment">;; QUESTION SECTION:</div>
            <div v-for="q in questionList" :key="q.name" class="terminal-line indent">
              ;{{ q.name }}&nbsp;&nbsp;&nbsp;&nbsp;IN&nbsp;&nbsp;&nbsp;&nbsp;{{ q.type }}
            </div>

            <div class="terminal-line mt-10 comment">;; ANSWER SECTION:</div>
            <div v-if="answerList.length > 0">
              <div v-for="(a, idx) in answerList" :key="idx" class="terminal-line indent highlight">
                <span class="lbl-domain">{{ a.name }}</span>&nbsp;&nbsp;&nbsp;&nbsp;<span class="lbl-ttl">{{ a.ttl }}</span>&nbsp;&nbsp;&nbsp;&nbsp;IN&nbsp;&nbsp;&nbsp;&nbsp;<span class="lbl-type">{{ a.type }}</span>&nbsp;&nbsp;&nbsp;&nbsp;<span class="lbl-data">{{ a.data }}</span>
              </div>
            </div>
            <div v-else class="terminal-line indent warning">
              ; [没有查询到符合该类型的解析记录]
            </div>
          </template>
          
          <template v-else-if="loading">
            <div class="terminal-line loading-line">;; Querying name servers for "{{ domain }}"...</div>
          </template>

          <template v-else>
            <div class="terminal-line empty-line">;; 等待发起域名解析查询...</div>
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="scss">
.tool-container {
  display: flex;
  flex-direction: column;
  height: 100%;
  width: 100%;
  padding: 12px;
  background: var(--jc-bg-app);
  overflow: hidden;
}
.tool-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
  flex-shrink: 0;
}
.tool-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--jc-text-highlight);
}
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 16px;
  min-height: 0;
}

/* 左侧控制栏 */
.control-panel {
  display: flex;
  flex-direction: column;
  flex: 0 0 320px;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 14px;
  border-radius: 4px;
  gap: 16px;
  overflow-y: auto;
}

.setting-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.section-subtitle {
  font-size: 11px;
  font-weight: 700;
  color: var(--jc-text-primary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  border-left: 2px solid var(--jc-color-accent);
  padding-left: 6px;
  line-height: 1.2;
}

.input-row {
  display: flex;
  gap: 8px;
}
.domain-input {
  flex: 1;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-family: inherit;
  font-size: 12px;
  padding: 6px 10px;
  outline: none;
  border-radius: 3px;
  height: 30px;
  min-width: 0;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.type-select {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  padding: 0 8px;
  font-size: 11px;
  outline: none;
  border-radius: 3px;
  height: 30px;
  cursor: pointer;
  width: 95px;
}

.doh-choice-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 4px 0;
}
.radio-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--jc-text-primary);
  cursor: pointer;
  input[type="radio"] {
    accent-color: var(--jc-color-accent);
    margin: 0;
  }
}

/* 右侧终端输出 */
.result-display-panel {
  display: flex;
  flex-direction: column;
  flex: 1;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 14px;
  border-radius: 4px;
  min-height: 0;
}
.pane-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
  font-weight: 600;
  margin-bottom: 10px;
  flex-shrink: 0;
}
.acts {
  display: flex;
  gap: 6px;
}

.terminal-view {
  flex: 1;
  background: #141416;
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 4px;
  padding: 12px;
  font-family: 'Cascadia Code', Consolas, Monaco, monospace;
  font-size: 12px;
  color: #c5c6c9;
  overflow-y: auto;
  line-height: 1.6;
}

.terminal-line {
  white-space: pre-wrap;
  word-break: break-all;
  
  &.comment {
    color: #5c6370;
  }
  &.indent {
    padding-left: 12px;
  }
  &.highlight {
    color: #e5c07b;
  }
  &.loading-line {
    color: var(--jc-color-accent);
  }
  &.empty-line {
    color: var(--jc-text-secondary);
  }
}

.status-val {
  color: var(--jc-color-success);
  font-weight: bold;
  &.error {
    color: var(--jc-color-error);
  }
}

.lbl-domain { color: #61afef; }
.lbl-ttl { color: #abb2bf; }
.lbl-type { color: #c678dd; font-weight: bold; }
.lbl-data { color: #98c379; font-weight: bold; }

.tool-btn {
  background: var(--jc-bg-btn);
  color: var(--jc-text-primary);
  border: none;
  padding: 4px 12px;
  font-size: 11px;
  cursor: pointer;
  border-radius: 2px;
  transition: all 0.2s;
  &:hover:not(:disabled) {
    background: var(--jc-bg-btn-hover);
  }
  &:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  &.pri {
    background: var(--jc-color-accent);
    color: var(--jc-color-white);
    &:hover {
      background: var(--jc-color-accent-hover);
    }
  }
  &.full {
    width: 100%;
  }
  &.large {
    padding: 8px 12px;
    font-size: 12px;
    font-weight: 600;
  }
  &.small {
    padding: 2px 8px;
    font-size: 10px;
  }
}

.tool-footer-error.style-inline {
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  border-radius: 2px;
}

.mt-10 { margin-top: 10px; }
.warning { color: var(--jc-color-warning); }
</style>
