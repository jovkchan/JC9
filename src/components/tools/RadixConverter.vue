<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'

const binVal = ref('')
const octVal = ref('')
const decVal = ref('')
const hexVal = ref('')

function updateValues(source: 'bin' | 'oct' | 'dec' | 'hex', rawValue: string) {
  if (!rawValue) {
    binVal.value = ''
    octVal.value = ''
    decVal.value = ''
    hexVal.value = ''
    return
  }

  try {
    let clean = ''
    let bigNum = 0n

    if (source === 'bin') {
      clean = rawValue.replace(/[^01]/g, '')
      binVal.value = clean // 保持用户输入的原样（包括前导0）
      if (!clean) return
      bigNum = BigInt('0b' + clean)
      
      octVal.value = bigNum.toString(8)
      decVal.value = bigNum.toString(10)
      hexVal.value = bigNum.toString(16).toUpperCase()
    } else if (source === 'oct') {
      clean = rawValue.replace(/[^0-7]/g, '')
      octVal.value = clean
      if (!clean) return
      bigNum = BigInt('0o' + clean)
      
      binVal.value = bigNum.toString(2)
      decVal.value = bigNum.toString(10)
      hexVal.value = bigNum.toString(16).toUpperCase()
    } else if (source === 'dec') {
      clean = rawValue.replace(/[^0-9]/g, '')
      decVal.value = clean
      if (!clean) return
      bigNum = BigInt(clean)
      
      binVal.value = bigNum.toString(2)
      octVal.value = bigNum.toString(8)
      hexVal.value = bigNum.toString(16).toUpperCase()
    } else if (source === 'hex') {
      clean = rawValue.replace(/[^0-9a-fA-F]/g, '')
      hexVal.value = clean
      if (!clean) return
      bigNum = BigInt('0x' + clean)
      
      binVal.value = bigNum.toString(2)
      octVal.value = bigNum.toString(8)
      decVal.value = bigNum.toString(10)
    }
  } catch (e) {
    // 捕获可能的大数解析错误
    console.error('进制转换大数解析失败', e)
  }
}

function copyText(text: string) {
  if (!text) return
  navigator.clipboard.writeText(text)
}

function clearAll() {
  binVal.value = ''
  octVal.value = ''
  decVal.value = ''
  hexVal.value = ''
}
</script>

<template>
  <ToolShell title="多进制转换器" subtitle="BIN / OCT / DEC / HEX 互转，支持任意长度大整数">
    <template #actions>
      <JcButton danger size="small" @click="clearAll">一键清空</JcButton>
    </template>

    <div class="info-alert">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4M12 8h.01"/></svg>
      <span>支持任意长度大整型数的无损精度转换（基于 JavaScript BigInt 原生驱动，可用于网络字节、IP地址、协议常量的转换校验）。</span>
    </div>

    <div class="radix-inputs-list">
      <div class="radix-field-row">
        <div class="radix-label">
          <span class="badge red">BIN</span>
          <span class="name">二进制 (Radix 2)</span>
        </div>
        <div class="input-wrap">
          <JcInput beam class="code-font" :model-value="binVal" @update:model-value="updateValues('bin', $event)" placeholder="请输入二进制数据（如 101010）..." style="flex: 1; min-width: 0" />
          <JcButton size="small" :disabled="!binVal" @click="copyText(binVal)" title="复制二进制">复制</JcButton>
        </div>
      </div>

      <div class="radix-field-row">
        <div class="radix-label">
          <span class="badge yellow">OCT</span>
          <span class="name">八进制 (Radix 8)</span>
        </div>
        <div class="input-wrap">
          <JcInput beam class="code-font" :model-value="octVal" @update:model-value="updateValues('oct', $event)" placeholder="请输入八进制数据（如 52）..." style="flex: 1; min-width: 0" />
          <JcButton size="small" :disabled="!octVal" @click="copyText(octVal)" title="复制八进制">复制</JcButton>
        </div>
      </div>

      <div class="radix-field-row">
        <div class="radix-label">
          <span class="badge purple">DEC</span>
          <span class="name">十进制 (Radix 10)</span>
        </div>
        <div class="input-wrap">
          <JcInput beam class="code-font" :model-value="decVal" @update:model-value="updateValues('dec', $event)" placeholder="请输入十进制数据（如 42）..." style="flex: 1; min-width: 0" />
          <JcButton size="small" :disabled="!decVal" @click="copyText(decVal)" title="复制十进制">复制</JcButton>
        </div>
      </div>

      <div class="radix-field-row">
        <div class="radix-label">
          <span class="badge green">HEX</span>
          <span class="name">十六进制 (Radix 16)</span>
        </div>
        <div class="input-wrap">
          <JcInput beam class="code-font" :model-value="hexVal" @update:model-value="updateValues('hex', $event)" placeholder="请输入十六进制数据（如 2A）..." style="flex: 1; min-width: 0" />
          <JcButton size="small" :disabled="!hexVal" @click="copyText(hexVal)" title="复制十六进制">复制</JcButton>
        </div>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped lang="scss">
.info-alert {
  font-size: 11px;
  color: var(--jc-text-secondary);
  background: rgba(138, 88, 255, 0.08);
  border-left: 3px solid var(--jc-color-accent);
  padding: 8px 12px;
  display: flex;
  gap: 8px;
  align-items: center;
  border-radius: 2px;
  svg {
    color: var(--jc-color-accent);
    flex-shrink: 0;
  }
}

.radix-inputs-list {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.radix-field-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.radix-label {
  display: flex;
  align-items: center;
  gap: 8px;
  .name {
    font-size: 11px;
    font-weight: 600;
    color: var(--jc-text-secondary);
  }
}
.badge {
  font-size: 9px;
  font-weight: 700;
  color: #fff;
  padding: 1px 6px;
  border-radius: 3px;
  width: 36px;
  text-align: center;
  &.red { background: #f44747; }
  &.yellow { background: #d7ba7d; color: #1e1e1e; }
  &.purple { background: var(--jc-color-accent); }
  &.green { background: #4ec9b0; }
}

.input-wrap {
  display: flex;
  gap: 8px;
}
.code-font {
  font-family: 'Cascadia Code', Consolas, monospace;
  letter-spacing: 0.5px;
}
</style>
