<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { createHighlighter, type Highlighter } from 'shiki'
import ConfirmDialog from './ConfirmDialog.vue'
import InfoTip from './InfoTip.vue'
import { getPrompt, savePrompt } from '../api'
import { dateLocale } from '../i18n'

const emit = defineEmits<{
  (e: 'notify', msg: string): void
  (e: 'error', msg: string): void
}>()

const { t } = useI18n()

const content = ref('')
const isDefault = ref(true)
const updatedAt = ref<string | null>(null)
const loading = ref(false)
const saving = ref(false)
const confirmClear = ref(false)

async function save() {
  saving.value = true
  try {
    const p = await savePrompt(content.value)
    content.value = p.content
    isDefault.value = p.is_default
    updatedAt.value = p.updated_at
    emit('notify', isDefault.value ? t('prompt.cleared') : t('prompt.saved'))
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    saving.value = false
  }
}

async function copy() {
  if (!content.value) {
    emit('notify', t('prompt.emptyWarn'))
    return
  }
  try {
    await navigator.clipboard.writeText(content.value)
    emit('notify', t('prompt.copied'))
  } catch {
    emit('notify', t('mcp.copyFailed'))
  }
}

/** 清空：保存空内容，回到默认空提示词 */
async function clearAll() {
  confirmClear.value = false
  content.value = ''
  await save()
}

function formatTime(iso: string | null): string {
  if (!iso) return ''
  const d = new Date(iso)
  return isNaN(d.getTime()) ? iso : d.toLocaleString(dateLocale.value, { dateStyle: 'short', timeStyle: 'short' })
}

// ---------- Markdown 实时高亮（Shiki 双主题，与「Agent 接入」页同套） ----------

/** 高亮层 HTML；引擎加载失败时为空，编辑器退回纯文本 */
const mdHtml = ref('')
const hlLayer = ref<HTMLElement | null>(null)
let highlighter: Highlighter | null = null
let hlTimer: number | undefined

/** 防抖重渲染：输入停顿后再跑 Shiki，避免每次击键都整段高亮 */
function scheduleHighlight() {
  window.clearTimeout(hlTimer)
  hlTimer = window.setTimeout(renderHighlighted, 200)
}

function renderHighlighted() {
  if (!highlighter) return
  // 末尾补一个换行：pre 不渲染最后的空行，会导致与 textarea 行对不齐
  mdHtml.value = highlighter.codeToHtml(content.value + '\n', {
    lang: 'markdown',
    themes: { light: 'github-light', dark: 'github-dark' },
    // 只输出 light/dark CSS 变量（同 MCPView，主题切换 CSS 见其全局样式）
    defaultColor: false,
  })
}

/** textarea 滚动同步到高亮层 */
function syncScroll(e: Event) {
  if (!hlLayer.value) return
  const ta = e.target as HTMLElement
  hlLayer.value.scrollTop = ta.scrollTop
  hlLayer.value.scrollLeft = ta.scrollLeft
}

onBeforeUnmount(() => window.clearTimeout(hlTimer))

watch(content, scheduleHighlight)

onMounted(async () => {
  try {
    highlighter = await createHighlighter({
      themes: ['github-light', 'github-dark'],
      langs: ['markdown'],
    })
    renderHighlighted()
  } catch {
    mdHtml.value = ''
  }
  loading.value = true
  try {
    const p = await getPrompt()
    content.value = p.content
    isDefault.value = p.is_default
    updatedAt.value = p.updated_at
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <h2 class="text-h6">{{ t('prompt.title') }}</h2>
      <InfoTip>
        {{ t('prompt.desc1') }}
        <span class="font-mono">prompt_get</span> /
        <span class="font-mono">prompt_update</span>
        {{ t('prompt.desc2') }}
      </InfoTip>
      <v-spacer />
      <v-btn variant="text" prepend-icon="mdi-content-copy" :disabled="!content" @click="copy">{{ t('prompt.copy') }}</v-btn>
      <v-btn
        variant="text"
        color="error"
        prepend-icon="mdi-eraser"
        :disabled="isDefault || loading"
        @click="confirmClear = true"
      >
        {{ t('prompt.clear') }}
      </v-btn>
    </div>

    <!-- 叠加编辑器：Shiki 高亮层在下、透明文字的 textarea 在上（行宽与换行规则严格一致）；
         高亮不可用时只有 textarea，退回纯文本编辑 -->
    <div class="prompt-editor mb-3">
      <pre
        v-if="mdHtml"
        ref="hlLayer"
        class="prompt-hl shiki"
        aria-hidden="true"
        v-html="mdHtml"
      ></pre>
      <textarea
        v-model="content"
        class="prompt-input"
        :disabled="loading"
        :aria-label="t('prompt.title')"
        :placeholder="t('prompt.placeholder')"
        spellcheck="false"
        @scroll="syncScroll"
      ></textarea>
      <v-progress-linear v-if="loading" indeterminate absolute bottom />
    </div>

    <div class="d-flex align-center">
      <v-btn
        color="primary"
        prepend-icon="mdi-content-save"
        :loading="saving"
        :disabled="loading"
        @click="save"
      >
        {{ t('common.save') }}
      </v-btn>
      <span class="text-caption text-medium-emphasis ml-3">
        {{ isDefault ? t('prompt.notSet') : t('prompt.customized', { time: formatTime(updatedAt) }) }}
      </span>
    </div>

    <ConfirmDialog
      v-model="confirmClear"
      :message="t('confirm.clearPrompt')"
      @confirm="clearAll"
    />
  </div>
</template>

<style scoped>
/* 叠加编辑器：两层必须使用完全一致的字体度量与换行规则，保证行对齐 */
.prompt-editor {
  position: relative;
  height: 480px;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 8px;
  background: rgb(var(--v-theme-surface));
  overflow: hidden;
}
.prompt-hl,
.prompt-input {
  margin: 0;
  padding: 12px;
  font-family: 'Monaspace Neon', 'JetBrains Mono', Consolas, 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.7;
  tab-size: 4;
  white-space: pre-wrap;
  overflow-wrap: break-word;
  word-break: break-word;
}
/* 高亮层：绝对铺满容器，仅随 textarea 滚动联动 */
.prompt-hl {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
}
/* 输入层：文字透明（透出下层高亮），光标与选区保持可见 */
.prompt-input {
  position: relative;
  z-index: 1;
  display: block;
  width: 100%;
  height: 100%;
  border: none;
  outline: none;
  resize: none;
  background: transparent;
  color: transparent;
  caret-color: rgb(var(--v-theme-primary));
}
.prompt-input::placeholder {
  color: rgba(var(--v-theme-on-surface), 0.45);
}
</style>
