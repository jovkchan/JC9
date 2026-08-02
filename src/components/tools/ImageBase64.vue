<script setup lang="ts">
import { ref } from 'vue'
import ToolShell from '@/components/ui/ToolShell.vue'
import JcButton from '@/components/ui/JcButton.vue'
import JcInput from '@/components/ui/JcInput.vue'
import JcTextarea from '@/components/ui/JcTextarea.vue'
import JcSegmented from '@/components/ui/JcSegmented.vue'

const activeTab = ref<'to-base64' | 'to-image'>('to-base64')

const tabOptions = [
  { label: '图片转 Base64', value: 'to-base64' },
  { label: 'Base64 还原图片', value: 'to-image' }
]

// 图片转 Base64 状态
const fileInput = ref<HTMLInputElement | null>(null)
const imageFile = ref<File | null>(null)
const imagePreview = ref('')
const imageWidth = ref(0)
const imageHeight = ref(0)
const imageSize = ref(0)
const imageMime = ref('')
const base64Pure = ref('')
const base64DataUrl = ref('')
const base64Html = ref('')
const base64Css = ref('')
const isDragging = ref(false)

// Base64 还原图片状态
const base64Input = ref('')
const decodedImageSrc = ref('')
const decodedWidth = ref(0)
const decodedHeight = ref(0)
const decodedSize = ref(0)
const decodedMime = ref('')
const decodeError = ref('')

function triggerFileInput() {
  fileInput.value?.click()
}

// 处理文件读取
function handleFile(file: File) {
  if (!file.type.startsWith('image/')) {
    alert('请选择有效的图片文件！')
    return
  }
  imageFile.value = file
  imageSize.value = file.size
  imageMime.value = file.type

  const reader = new FileReader()
  reader.onload = (e) => {
    const dataUrl = e.target?.result as string
    base64DataUrl.value = dataUrl
    
    // 提取纯 Base64 部分
    const pure = dataUrl.split(',')[1] || ''
    base64Pure.value = pure

    // 格式化 HTML 和 CSS 引用
    base64Html.value = `<img src="${dataUrl}" alt="image" />`
    base64Css.value = `background-image: url("${dataUrl}");`
    imagePreview.value = dataUrl

    // 读取分辨率
    const img = new Image()
    img.onload = () => {
      imageWidth.value = img.width
      imageHeight.value = img.height
    }
    img.src = dataUrl
  }
  reader.readAsDataURL(file)
}

function onFileSelect(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files[0]) {
    handleFile(target.files[0])
  }
}

function onDragOver(e: DragEvent) {
  e.preventDefault()
  isDragging.value = true
}

function onDragLeave() {
  isDragging.value = false
}

function onDrop(e: DragEvent) {
  e.preventDefault()
  isDragging.value = false
  if (e.dataTransfer?.files && e.dataTransfer.files[0]) {
    handleFile(e.dataTransfer.files[0])
  }
}

// 还原 Base64 到图片
function handleDecode() {
  decodeError.value = ''
  decodedImageSrc.value = ''
  decodedWidth.value = 0
  decodedHeight.value = 0
  decodedSize.value = 0
  decodedMime.value = ''

  let inputStr = base64Input.value.trim()
  if (!inputStr) return

  let dataUrl = inputStr
  if (!inputStr.startsWith('data:')) {
    dataUrl = 'data:image/png;base64,' + inputStr
  }

  const parts = dataUrl.split(',')
  if (parts.length < 2 || !parts[0].includes('base64')) {
    decodeError.value = 'Base64 数据格式错误或不支持'
    return
  }

  try {
    const pureBase64 = parts[1]
    atob(pureBase64)
  } catch (e) {
    decodeError.value = 'Base64 字符串解码失败，数据可能不完整'
    return
  }

  decodedImageSrc.value = dataUrl
  const pureBase64 = parts[1]
  decodedSize.value = Math.round(pureBase64.length * 0.75)

  const mimeMatch = parts[0].match(/:(.*?);/)
  decodedMime.value = mimeMatch ? mimeMatch[1] : 'image/png'

  const img = new Image()
  img.onload = () => {
    decodedWidth.value = img.width
    decodedHeight.value = img.height
  }
  img.onerror = () => {
    decodeError.value = '无法加载图片，Base64 数据可能损坏'
    decodedImageSrc.value = ''
  }
  img.src = dataUrl
}

// 保存图片
function downloadDecodedImage() {
  if (!decodedImageSrc.value) return
  
  try {
    const arr = decodedImageSrc.value.split(',')
    const mime = arr[0].match(/:(.*?);/)?.[1] || 'image/png'
    const bstr = atob(arr[1])
    let n = bstr.length
    const u8arr = new Uint8Array(n)
    while (n--) {
      u8arr[n] = bstr.charCodeAt(n)
    }
    const blob = new Blob([u8arr], { type: mime })
    const blobUrl = URL.createObjectURL(blob)

    const a = document.createElement('a')
    a.href = blobUrl
    const ext = mime.split('/')[1] || 'png'
    a.download = `decoded_image_${Date.now()}.${ext}`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(blobUrl)
  } catch (e) {
    alert('保存图片失败')
  }
}

// 复制
function copyText(text: string) {
  if (!text) return
  navigator.clipboard.writeText(text)
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B'
  const k = 1024
  const sizes = ['B', 'KB', 'MB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

function clearToConverter() {
  imageFile.value = null
  imagePreview.value = ''
  imageWidth.value = 0
  imageHeight.value = 0
  imageSize.value = 0
  imageMime.value = ''
  base64Pure.value = ''
  base64DataUrl.value = ''
  base64Html.value = ''
  base64Css.value = ''
}

function clearDecoder() {
  base64Input.value = ''
  decodedImageSrc.value = ''
  decodedWidth.value = 0
  decodedHeight.value = 0
  decodedSize.value = 0
  decodedMime.value = ''
  decodeError.value = ''
}
</script>

<template>
  <ToolShell title="图片 Base64 互转">
    <template #actions>
      <JcSegmented
        :model-value="activeTab"
        :options="tabOptions"
        size="small"
        @update:model-value="(v) => activeTab = v as 'to-base64' | 'to-image'"
      />
    </template>

    <!-- Tab 1: 图片转 Base64 (重构后的扁平上下垂直结构) -->
    <div v-if="activeTab === 'to-base64'" class="tool-body-vertical">
      <!-- 扁平化拖拽上传区 / 状态卡片 -->
      <div 
        class="flat-drop-zone"
        :class="{ dragging: isDragging, uploaded: imagePreview }"
        @dragover="onDragOver"
        @dragleave="onDragLeave"
        @drop="onDrop"
        @click="!imagePreview && triggerFileInput()"
      >
        <input type="file" ref="fileInput" @change="onFileSelect" accept="image/*" class="file-input-hidden" />
        
        <!-- 未上传状态：宽条上传按钮 -->
        <div v-if="!imagePreview" class="flat-drop-content">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="upload-icon"><rect width="18" height="18" x="3" y="3" rx="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>
          <span class="upload-msg">将图片拖拽到此处，或 <strong class="click-trigger">点击上传</strong></span>
          <span class="upload-tip">支持 PNG, JPG, GIF, WEBP, SVG 格式</span>
        </div>

        <!-- 已上传状态：超扁平卡片横条 -->
        <div v-else class="flat-uploaded-content" @click.stop>
          <div class="flat-left-meta">
            <div class="flat-thumb-wrap">
              <img :src="imagePreview" alt="thumbnail" />
            </div>
            <div class="flat-meta-details">
              <span class="flat-filename" :title="imageFile?.name">{{ imageFile?.name || '已载入图片' }}</span>
              <span class="flat-specs">
                格式: <strong>{{ imageMime.split('/')[1]?.toUpperCase() }}</strong> · 
                尺寸: <strong>{{ imageWidth }} x {{ imageHeight }} Px</strong> · 
                大小: <strong>{{ formatBytes(imageSize) }}</strong>
              </span>
            </div>
          </div>
          <JcButton size="small" danger ghost @click="clearToConverter" title="清除并重新上传">✕ 重新上传</JcButton>
        </div>
      </div>

      <!-- 剩余撑满的 Base64 编码输出区域 -->
      <div class="flat-results-pane">
        <div class="copy-text-group flex-fill-group">
          <div class="group-title">
            <span>Data URL 格式 (用于 CSS / HTML 直引)</span>
            <JcButton size="small" type="primary" @click="copyText(base64DataUrl)" :disabled="!base64DataUrl">复制</JcButton>
          </div>
          <JcTextarea :model-value="base64DataUrl" mono readonly class="jc-fill" placeholder="上传图片后自动生成..." />
        </div>

        <div class="copy-text-group flex-fill-group">
          <div class="group-title">
            <span>纯 Base64 数据</span>
            <JcButton size="small" @click="copyText(base64Pure)" :disabled="!base64Pure">复制</JcButton>
          </div>
          <JcTextarea :model-value="base64Pure" mono readonly class="jc-fill" placeholder="上传图片后自动生成..." />
        </div>

        <div class="copy-text-group flex-row">
          <div class="flex-item">
            <div class="group-title">
              <span>HTML Image 标签</span>
              <JcButton size="small" @click="copyText(base64Html)" :disabled="!base64Html">复制</JcButton>
            </div>
            <JcInput :model-value="base64Html" readonly placeholder="等待生成..." style="font-family: 'Cascadia Code', Consolas, monospace" />
          </div>
          <div class="flex-item">
            <div class="group-title">
              <span>CSS Background 声明</span>
              <JcButton size="small" @click="copyText(base64Css)" :disabled="!base64Css">复制</JcButton>
            </div>
            <JcInput :model-value="base64Css" readonly placeholder="等待生成..." style="font-family: 'Cascadia Code', Consolas, monospace" />
          </div>
        </div>
      </div>
    </div>

    <!-- Tab 2: Base64 还原图片 -->
    <div v-else class="tool-body-split">
      <div class="input-pane">
        <div class="pane-label-bar">
          <span>粘贴 Base64 字符串</span>
          <div class="pane-acts">
            <JcButton type="primary" size="small" @click="handleDecode" :disabled="!base64Input.trim()">解析还原</JcButton>
            <JcButton size="small" danger ghost @click="clearDecoder">清空</JcButton>
          </div>
        </div>
        <JcTextarea v-model="base64Input" @input="handleDecode" mono :spellcheck="false" class="jc-fill" placeholder="在此处粘贴图片的 Base64 字符串（支持包含或不包含前缀 data:image/...;base64,）..." />
        <div v-if="decodeError" class="tool-footer-error style-inline">{{ decodeError }}</div>
      </div>

      <div class="preview-decoded-pane">
        <div class="pane-label">预览及下载</div>
        <div class="decode-image-box">
          <div v-if="!decodedImageSrc" class="empty-preview">
            <svg viewBox="0 0 24 24" width="36" height="36" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>
            <p>等待解析 Base64 渲染图像...</p>
          </div>
          <img v-else :src="decodedImageSrc" alt="decoded-preview" />
        </div>
        <div v-if="decodedImageSrc" class="decode-meta-info">
          <div class="meta-details">
            <div>格式: <strong>{{ decodedMime }}</strong></div>
            <div>尺寸: <strong>{{ decodedWidth }} x {{ decodedHeight }} Px</strong></div>
            <div>预估大小: <strong>{{ formatBytes(decodedSize) }}</strong></div>
          </div>
          <JcButton type="primary" block @click="downloadDecodedImage">下载图片到本地</JcButton>
        </div>
      </div>
    </div>
  </ToolShell>
</template>

<style scoped lang="scss">
.file-input-hidden {
  position: absolute;
  width: 0;
  height: 0;
  opacity: 0;
  pointer-events: none;
}

/* ================= Tab 1: 新扁平上下垂直结构 ================= */
.tool-body-vertical {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  gap: 12px;
}

/* 扁平拖拽条 */
.flat-drop-zone {
  flex-shrink: 0;
  height: 66px;
  border: 2px dashed var(--jc-border-strong);
  background: var(--jc-bg-panel);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 16px;
  transition: all 0.2s;
  &:hover, &.dragging {
    border-color: var(--jc-color-accent);
    background: var(--jc-bg-hover);
  }
  &.uploaded {
    border-style: solid;
    border-color: var(--jc-border-default);
    cursor: default;
    background: var(--jc-bg-panel);
    &:hover {
      background: var(--jc-bg-panel);
    }
  }
}

.flat-drop-content {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  width: 100%;
  color: var(--jc-text-secondary);
  
  .upload-icon {
    color: var(--jc-color-accent);
  }
  .upload-msg {
    font-size: 12px;
    color: var(--jc-text-primary);
    .click-trigger {
      color: var(--jc-color-accent);
      text-decoration: underline;
      cursor: pointer;
    }
  }
  .upload-tip {
    font-size: 10px;
    color: var(--jc-text-secondary);
    margin-left: auto;
  }
}

/* 扁平已上传信息条 */
.flat-uploaded-content {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 100%;
}
.flat-left-meta {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}
.flat-thumb-wrap {
  width: 44px;
  height: 44px;
  border-radius: 4px;
  border: 1px solid var(--jc-border-strong);
  background: var(--jc-bg-app);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2);
  img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
}
.flat-meta-details {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.flat-filename {
  font-size: 12px;
  font-weight: 600;
  color: var(--jc-text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.flat-specs {
  font-size: 11px;
  color: var(--jc-text-secondary);
  strong {
    color: var(--jc-text-primary);
  }
}

/* 纵向伸展的 Base64 编码区 */
.flat-results-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}
.copy-text-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
  &.flex-fill-group {
    flex: 1;
    min-height: 0;
  }
  &.flex-row {
    flex-direction: row;
    gap: 12px;
    flex-shrink: 0;
  }
}
.flex-item {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.group-title {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
  font-weight: 600;
}

/* ================= Tab 2: 原有 Base64 还原布局不变 ================= */
.tool-body-split {
  display: flex;
  flex: 1;
  gap: 16px;
  min-height: 0;
}
.input-pane {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  height: 100%;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 10px;
  border-radius: 4px;
}
.pane-label-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--jc-text-secondary);
  font-weight: 600;
  margin-bottom: 8px;
}
.pane-acts {
  display: flex;
  gap: 6px;
}

.preview-decoded-pane {
  display: flex;
  flex-direction: column;
  flex: 0 0 300px;
  background: var(--jc-bg-panel);
  border: 1px solid var(--jc-border-default);
  padding: 10px;
  border-radius: 4px;
}
.pane-label {
  font-size: 11px;
  color: var(--jc-text-secondary);
  margin-bottom: 8px;
  text-transform: uppercase;
}
.decode-image-box {
  flex: 1;
  background: var(--jc-bg-app);
  border: 1px solid var(--jc-border-strong);
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  padding: 8px;
  img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 2px;
    box-shadow: 0 2px 8px rgba(0,0,0,0.2);
  }
}
.empty-preview {
  text-align: center;
  color: var(--jc-text-secondary);
  svg {
    margin: 0 auto 10px auto;
    opacity: 0.3;
  }
  p {
    font-size: 11px;
    margin: 0;
  }
}
.decode-meta-info {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.meta-details {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 11px;
  color: var(--jc-text-secondary);
  strong {
    color: var(--jc-text-primary);
  }
}

.tool-footer-error.style-inline {
  flex-shrink: 0;
  margin-top: 8px;
  font-size: 11px;
  color: var(--jc-color-error);
  background: rgba(244, 71, 71, 0.1);
  padding: 6px 12px;
  border-left: 3px solid var(--jc-color-error);
  font-family: inherit;
}
</style>
