<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInputNumber from '@/components/ui/JcInputNumber.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'

const language = ref<'cn' | 'en'>('cn')
const paragraphs = ref(3)
const textLength = ref(150) // 每段大致字数/单词数
const includeHtml = ref(false)
const output = ref('')

const CHINESE_SENTENCES = [
  "落霞与孤鹜齐飞，秋水共长天一色。",
  "明月出天山，苍茫云海间。",
  "行到水穷处，坐看云起时。",
  "白发渔樵江渚上，惯看秋月春风。",
  "小桥流水人家，古道西风瘦马。",
  "海内存知己，天涯若比邻。",
  "大漠孤烟直，长河落日圆。",
  "随风潜入夜，润物细无声。",
  "两岸猿声啼不住，轻舟已过万重山。",
  "明月松间照，清泉石上流。",
  "山重水复疑无路，柳暗花明又一村。",
  "潮平两岸阔，风正一帆悬。",
  "星垂平野阔，月涌大江流。",
  "无边落木萧萧下，不尽长江滚滚来。",
  "醉翁之意不在酒，在乎山水之间也。",
  "先天下之忧而忧，后天下之乐而乐。",
  "金沙水拍云崖暖，大渡桥横铁索寒。",
  "雄关漫道真如铁，而今迈步从头越。",
  "江山如此多娇，引无数英雄竞折腰。",
  "恰同学少年，风华正茂，书生意气，挥斥方遒。"
]

const LOREM_WORDS = [
  "lorem", "ipsum", "dolor", "sit", "amet", "consectetur", "adipiscing", "elit", 
  "sed", "do", "eiusmod", "tempor", "incididunt", "ut", "labore", "et", "dolore", 
  "magna", "aliqua", "ut", "enim", "ad", "minim", "veniam", "quis", "nostrud", 
  "exercitation", "ullamco", "laboris", "nisi", "ut", "aliquip", "ex", "ea", 
  "commodo", "consequat", "duis", "aute", "irure", "dolor", "in", "reprehenderit", 
  "in", "voluptate", "velit", "esse", "cillum", "dolore", "eu", "fugiat", "nulla", 
  "pariatur", "excepteur", "sint", "occaecat", "cupidatat", "non", "proident", 
  "sunt", "in", "culpa", "qui", "officia", "deserunt", "mollit", "anim", "id", "est", "laborum"
]

function generateText() {
  const result: string[] = []

  for (let p = 0; p < paragraphs.value; p++) {
    let pText = ''
    if (language.value === 'cn') {
      // 拼接中文句子直到达到字数
      while (pText.length < textLength.value) {
        const randIdx = Math.floor(Math.random() * CHINESE_SENTENCES.length)
        pText += CHINESE_SENTENCES[randIdx] + ' '
      }
      pText = pText.trim()
    } else {
      // 拼接英文单词
      const words: string[] = []
      for (let w = 0; w < textLength.value; w++) {
        const randIdx = Math.floor(Math.random() * LOREM_WORDS.length)
        let word = LOREM_WORDS[randIdx]
        // 首单词大写
        if (w === 0) {
          word = word.charAt(0).toUpperCase() + word.slice(1)
        }
        words.push(word)
      }
      pText = words.join(' ') + '.'
    }

    if (includeHtml.value) {
      result.push(`<p>${pText}</p>`)
    } else {
      result.push(pText)
    }
  }

  output.value = result.join(includeHtml.value ? '\n' : '\n\n')
}

// 自动生成初始占位文
generateText()

function copyText() {
  if (!output.value) return
  navigator.clipboard.writeText(output.value)
}
</script>

<template>
  <ToolShell title="占位符文本生成器" subtitle="Lorem Ipsum" split>
    <template #actions>
      <JcButton type="primary" size="small" @click="generateText">重新生成</JcButton>
      <JcButton type="primary" size="small" :disabled="!output" @click="copyText">复制文本</JcButton>
    </template>

    <template #left-label>基本配置</template>
    <template #left>
      <div class="setting-section">
        <div class="config-field">
          <label>语言模式</label>
          <div class="radio-group">
            <label class="radio-label">
              <input type="radio" value="cn" v-model="language" @change="generateText" />
              <span>优雅中文</span>
            </label>
            <label class="radio-label">
              <input type="radio" value="en" v-model="language" @change="generateText" />
              <span>经典英文 (Lorem)</span>
            </label>
          </div>
        </div>

        <div class="config-field mt-10">
          <label>段落数量 (Paragraphs)</label>
          <JcInputNumber :model-value="paragraphs" :min="1" :max="50" beam glow @update:model-value="paragraphs = $event ?? 1" @change="generateText" />
        </div>

        <div class="config-field mt-10">
          <label>{{ language === 'cn' ? '每段大致字数' : '每段大致单词数' }}</label>
          <JcInputNumber :model-value="textLength" :min="10" :max="1000" beam glow @update:model-value="textLength = $event ?? 10" @change="generateText" />
        </div>

        <div class="config-field mt-10">
          <label class="checkbox-label">
            <input type="checkbox" v-model="includeHtml" @change="generateText" />
            <span>包含 HTML &lt;p&gt; 标签</span>
          </label>
        </div>
      </div>
    </template>

    <template #right-label>生成的占位符文本</template>
    <template #right>
      <JcTextarea v-model="output" mono readonly beam glow :beam-size-ratio="0.6" :spellcheck="false" class="jc-fill" placeholder="等待生成..." />
    </template>
  </ToolShell>
</template>

<style scoped lang="scss">
.setting-section {
  display: flex;
  flex-direction: column;
  gap: 12px;
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

.config-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  label {
    font-size: 11px;
    color: var(--jc-text-secondary);
  }
}
.radio-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.radio-label, .checkbox-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--jc-text-primary);
  cursor: pointer;
  input[type="radio"], input[type="checkbox"] {
    accent-color: var(--jc-color-accent);
    margin: 0;
  }
}
</style>
