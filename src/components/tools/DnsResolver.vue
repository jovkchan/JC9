<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcSelect from '@/components/ui/JcSelect.vue'

const domain = ref('github.com')
const recordType = ref('A')
const dohProvider = ref<'cloudflare' | 'alidns'>('cloudflare')
const loading = ref(false)
const errorMsg = ref('')

const recordTypeOptions = [
  { label: 'A 记录', value: 'A' },
  { label: 'CNAME 记录', value: 'CNAME' },
  { label: 'AAAA 记录', value: 'AAAA' },
  { label: 'MX 记录', value: 'MX' },
  { label: 'TXT 记录', value: 'TXT' },
  { label: 'NS 记录', value: 'NS' }
]

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
  <ToolShell title="DNS 解析查询" subtitle="dig" split>
    <template #left-label>域名及类型设置</template>
    <template #left>
      <div class="control-panel">
        <div class="input-row">
          <JcInput
            beam glow
            v-model="domain"
            placeholder="请输入域名，如 github.com"
            style="flex: 1; min-width: 0"
            @keyup.enter="resolveDns"
          />
          <JcSelect beam glow v-model="recordType" :options="recordTypeOptions" style="width: 110px" />
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

        <JcButton type="primary" block size="large" :loading="loading" @click="resolveDns">
          {{ loading ? '正在进行 DNS 查询...' : '域名解析查询 (dig)' }}
        </JcButton>

        <div v-if="errorMsg" class="tool-footer-error style-inline">{{ errorMsg }}</div>
      </div>
    </template>

    <template #right-label>终端 dig 输出模拟</template>
    <template #right>
      <div class="result-display-panel">
        <div class="pane-header-row">
          <div class="acts">
            <JcButton size="small" :disabled="!rawJson" @click="copyAnswersText">复制记录</JcButton>
            <JcButton size="small" :disabled="!rawJson" @click="copyJson">复制 JSON</JcButton>
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
    </template>
  </ToolShell>
</template>

<style scoped lang="scss">
/* 左侧控制栏 */
.control-panel {
  display: flex;
  flex-direction: column;
  gap: 16px;
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
  min-height: 0;
}
.pane-header-row {
  display: flex;
  justify-content: flex-end;
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
