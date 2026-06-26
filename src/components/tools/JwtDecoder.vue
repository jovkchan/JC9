<script setup lang="ts">
import { ref, watch } from 'vue'

const tokenInput = ref('')
const headerResult = ref('')
const payloadResult = ref('')
const errorMsg = ref('')

// 时间戳解析结果
interface DecodedTime {
  claim: string
  label: string
  raw: number
  formatted: string
}
const decodedTimes = ref<DecodedTime[]>([])

// JWT Base64URL 乱码安全解码算法
function base64UrlDecode(str: string): string {
  // 1. 规范化 Base64URL 字符
  let base64 = str.replace(/-/g, '+').replace(/_/g, '/')
  
  // 2. 补齐末尾等号
  const pad = base64.length % 4
  if (pad) {
    base64 += '='.repeat(4 - pad)
  }
  
  // 3. atob 解码为字节流
  const raw = atob(base64)
  
  // 4. 利用 TextDecoder 按照 UTF-8 规范解码（防止中文乱码）
  const bytes = new Uint8Array(raw.length)
  for (let i = 0; i < raw.length; i++) {
    bytes[i] = raw.charCodeAt(i)
  }
  return new TextDecoder('utf-8').decode(bytes)
}

function decodeJwt() {
  headerResult.value = ''
  payloadResult.value = ''
  errorMsg.value = ''
  decodedTimes.value = []

  const token = tokenInput.value.trim()
  if (!token) return

  const parts = token.split('.')
  if (parts.length !== 3) {
    errorMsg.value = 'JWT 格式非法：有效的 JWT 必须包含由句点 (.) 分割的 3 个部分（Header.Payload.Signature）'
    return
  }

  try {
    // 解码 Header
    const headerJson = base64UrlDecode(parts[0])
    headerResult.value = JSON.stringify(JSON.parse(headerJson), null, 2)

    // 解码 Payload
    const payloadJson = base64UrlDecode(parts[1])
    const parsedPayload = JSON.parse(payloadJson)
    payloadResult.value = JSON.stringify(parsedPayload, null, 2)

    // 解析 exp, iat, nbf 时间戳
    const timeClaims = [
      { key: 'iat', label: '签发时间 (Issued At)' },
      { key: 'exp', label: '过期时间 (Expiration)' },
      { key: 'nbf', label: '生效时间 (Not Before)' }
    ]

    const times: DecodedTime[] = []
    for (const item of timeClaims) {
      if (parsedPayload && typeof parsedPayload[item.key] === 'number') {
        const rawVal = parsedPayload[item.key]
        // 自动识别秒级和毫秒级时间戳 (以 10000000000 为界限)
        const isSecond = rawVal < 10000000000
        const date = new Date(isSecond ? rawVal * 1000 : rawVal)
        
        if (!isNaN(date.getTime())) {
          const y = date.getFullYear()
          const m = String(date.getMonth() + 1).padStart(2, '0')
          const d = String(date.getDate()).padStart(2, '0')
          const hh = String(date.getHours()).padStart(2, '0')
          const mm = String(date.getMinutes()).padStart(2, '0')
          const ss = String(date.getSeconds()).padStart(2, '0')
          times.push({
            claim: item.key,
            label: item.label,
            raw: rawVal,
            formatted: `${y}-${m}-${d} ${hh}:${mm}:${ss}`
          })
        }
      }
    }
    decodedTimes.value = times

  } catch (e: any) {
    errorMsg.value = '解析失败：Header 或 Payload 不是合法的 Base64 / JSON 编码格式 (' + (e.message || '格式错误') + ')'
  }
}

watch(tokenInput, () => {
  decodeJwt()
})

function copyText(text: string) {
  if (!text) return
  navigator.clipboard.writeText(text)
}

function clearAll() {
  tokenInput.value = ''
  headerResult.value = ''
  payloadResult.value = ''
  errorMsg.value = ''
  decodedTimes.value = []
}
</script>

<template>
  <div class="tool-container">
    <div class="tool-header">
      <div class="tool-title">JWT 解码器</div>
      <div class="tool-actions">
        <button class="tool-btn err" @click="clearAll">清空</button>
      </div>
    </div>

    <!-- 上方：输入 Token -->
    <div class="jwt-input-section">
      <div class="pane-label">粘贴 JWT Token (三段式字符串)</div>
      <textarea v-model="tokenInput" placeholder="在此处粘贴以 eyJ... 开头的三段式 JWT 令牌..." class="token-textarea" spellcheck="false"></textarea>
    </div>

    <!-- 下方：Header 与 Payload 左右分栏 -->
    <div class="tool-body-split">
      <div class="editor-pane">
        <div class="pane-label-row">
          <span>Header (头部信息)</span>
          <button class="tool-btn pri small" @click="copyText(headerResult)" :disabled="!headerResult">复制 Header</button>
        </div>
        <textarea v-model="headerResult" readonly placeholder="等待解析..." class="readonly-output code-font" spellcheck="false"></textarea>
      </div>

      <div class="editor-pane flex-column-layout">
        <div class="pane-label-row">
          <span>Payload (有效载荷)</span>
          <button class="tool-btn pri small" @click="copyText(payloadResult)" :disabled="!payloadResult">复制 Payload</button>
        </div>
        <textarea v-model="payloadResult" readonly placeholder="等待解析..." class="readonly-output code-font flex-fill" spellcheck="false"></textarea>
        
        <!-- 时间戳声明解析看版 -->
        <div v-if="decodedTimes.length > 0" class="time-claims-panel">
          <div class="claims-title">时间戳声明转换</div>
          <div v-for="t in decodedTimes" :key="t.claim" class="claim-time-row">
            <span class="claim-badge">{{ t.claim }}</span>
            <span class="claim-lbl">{{ t.label }}：</span>
            <strong class="claim-val">{{ t.formatted }}</strong>
          </div>
        </div>
      </div>
    </div>
    <div v-if="errorMsg" class="tool-footer-error">{{ errorMsg }}</div>
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
.jwt-input-section {
  display: flex;
  flex-direction: column;
  margin-bottom: 12px;
  flex-shrink: 0;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 6px;
  text-transform: uppercase;
}
.token-textarea {
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-family: 'Cascadia Code', Consolas, monospace;
  font-size: 12px;
  padding: 8px;
  outline: none;
  border-radius: 4px;
  height: 70px;
  resize: none;
  width: 100%;
  &:focus {
    border-color: var(--jc-color-accent);
  }
}
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 12px;
  min-height: 0;
}
.editor-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  border: 1px solid var(--jc-border-default);
  background: var(--jc-bg-panel);
  padding: 8px;
  border-radius: 4px;
  &.flex-column-layout {
    display: flex;
    flex-direction: column;
  }
}
.pane-label-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 6px;
  text-transform: uppercase;
  flex-shrink: 0;
}
textarea {
  flex: 1;
  width: 100%;
  resize: none;
  background: var(--jc-bg-input);
  border: 1px solid var(--jc-border-strong);
  color: var(--jc-text-primary);
  font-size: 12px;
  padding: 8px;
  outline: none;
  border-radius: 2px;
  &:focus {
    border-color: var(--jc-color-accent);
  }
  &.code-font {
    font-family: 'Cascadia Code', Consolas, monospace;
  }
}
.flex-fill {
  flex: 1;
  min-height: 0;
}
.readonly-output {
  background: var(--jc-bg-app);
  color: var(--jc-color-success);
}
.time-claims-panel {
  margin-top: 8px;
  background: var(--jc-bg-app);
  border: 1px solid var(--jc-border-strong);
  border-radius: 3px;
  padding: 8px;
  flex-shrink: 0;
}
.claims-title {
  font-size: 10px;
  font-weight: 700;
  color: var(--jc-text-secondary);
  text-transform: uppercase;
  margin-bottom: 4px;
  border-bottom: 1px solid var(--jc-border-default);
  padding-bottom: 2px;
}
.claim-time-row {
  display: flex;
  align-items: center;
  font-size: 11px;
  line-height: 1.6;
}
.claim-badge {
  background: var(--jc-color-accent-light);
  color: var(--jc-color-accent-hover);
  font-size: 9px;
  font-weight: 700;
  padding: 0 4px;
  border-radius: 2px;
  margin-right: 6px;
  font-family: 'Cascadia Code', Consolas, monospace;
}
.claim-lbl {
  color: var(--jc-text-secondary);
}
.claim-val {
  color: var(--jc-color-success);
}

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
  &.err {
    &:hover {
      background: var(--jc-color-error);
      color: var(--jc-color-white);
    }
  }
  &.small {
    padding: 2px 8px;
    font-size: 10px;
  }
}
.tool-footer-error {
  flex-shrink: 0;
  margin-top: 8px;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: 'Cascadia Code', Consolas, monospace;
}
</style>
