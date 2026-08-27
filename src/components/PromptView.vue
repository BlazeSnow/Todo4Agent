<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { useTheme } from 'vuetify'
import type { Compartment } from '@codemirror/state'
import type { EditorView } from '@codemirror/view'
import ConfirmDialog from './ConfirmDialog.vue'
import InfoTip from './InfoTip.vue'
import { getPrompt, savePrompt } from '../api'
import { dateLocale } from '../i18n'

const emit = defineEmits<{
  (e: 'notify', msg: string): void
  (e: 'error', msg: string): void
}>()

const { t } = useI18n()
const vuetifyTheme = useTheme()

const content = ref('')
const isDefault = ref(true)
const updatedAt = ref<string | null>(null)
const loading = ref(false)
const saving = ref(false)
const confirmClear = ref(false)

// ---------- CodeMirror 6 Markdown 编辑器 ----------

// 动态加载 CodeMirror（仅本页需要，避免增大首屏包）
const editorHost = ref<HTMLElement | null>(null)
/** 编辑器初始化失败时退回纯文本 textarea */
const editorFailed = ref(false)
let view: EditorView | null = null
let themeComp: Compartment | null = null

/** 深浅主题扩展：跟随 Vuetify 主题切换 github 风格 light / oneDark */
async function buildExtensions(cmTheme: boolean) {
  const { EditorView, placeholder: cmPlaceholder } = await import('@codemirror/view')
  const { minimalSetup } = await import('codemirror')
  const { markdown } = await import('@codemirror/lang-markdown')
  const { oneDark } = await import('@codemirror/theme-one-dark')
  return [
    minimalSetup,
    markdown(),
    EditorView.lineWrapping,
    cmPlaceholder(t('prompt.placeholder')),
    // 固定高度容器内滚动；字体与「Agent 接入」页代码块一致
    EditorView.theme({
      '&': { height: '480px', fontSize: '13px' },
      '.cm-scroller': {
        fontFamily: "'Monaspace Neon', 'JetBrains Mono', Consolas, 'Courier New', monospace",
        lineHeight: '1.7',
      },
      '.cm-content': { caretColor: 'rgb(var(--v-theme-primary))' },
      '.cm-gutters': { userSelect: 'none' },
    }),
    themeComp!.of(cmTheme ? [oneDark] : []),
  ]
}

/** 创建编辑器（初始文档为已加载的提示词内容） */
async function initEditor(initial: string) {
  const { EditorView } = await import('@codemirror/view')
  const { EditorState, Compartment } = await import('@codemirror/state')
  themeComp = new Compartment()
  const exts = [
    await buildExtensions(vuetifyTheme.current.value.dark),
    EditorView.updateListener.of((u) => {
      if (u.docChanged && view) content.value = view.state.doc.toString()
    }),
  ]
  view = new EditorView({
    parent: editorHost.value!,
    state: EditorState.create({ doc: initial, extensions: exts }),
  })
}

/** 主题切换：重配深浅扩展 */
async function applyTheme() {
  if (!view || !themeComp) return
  view.dispatch({ effects: themeComp.reconfigure(await buildExtensions(vuetifyTheme.current.value.dark)) })
}

watch(content, (v) => {
  // 编辑器内部变更已在 updateListener 同步到 content（同值不触发）；
  // 这里只处理程序化修改（加载、清空）回写到编辑器
  if (view && view.state.doc.toString() !== v) {
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: v } })
  }
})

watch(
  () => vuetifyTheme.current.value.dark,
  () => applyTheme(),
)

onBeforeUnmount(() => {
  view?.destroy()
  view = null
})

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

onMounted(async () => {
  loading.value = true
  try {
    const p = await getPrompt()
    content.value = p.content
    isDefault.value = p.is_default
    updatedAt.value = p.updated_at
    await nextTickSafe()
    try {
      await initEditor(content.value)
    } catch {
      editorFailed.value = true
    }
  } catch (e) {
    emit('error', (e as Error).message)
    editorFailed.value = true
  } finally {
    loading.value = false
  }
})

/** 等一帧：编辑器挂载点需已在 DOM 中 */
async function nextTickSafe() {
  await new Promise((r) => requestAnimationFrame(() => r(null)))
}
</script>

<template>
  <div>
    <div class="d-flex align-center mb-4">
      <h2 class="text-h6">{{ t('prompt.title') }}</h2>
      <InfoTip class="ml-2">
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

    <!-- CodeMirror 6 Markdown 编辑器（动态加载）；初始化失败退回纯文本 -->
    <div class="prompt-editor mb-3">
      <div ref="editorHost" class="prompt-cm" :aria-label="t('prompt.title')"></div>
      <textarea
        v-if="editorFailed"
        v-model="content"
        class="prompt-fallback"
        :aria-label="t('prompt.title')"
        :placeholder="t('prompt.placeholder')"
        spellcheck="false"
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
.prompt-editor {
  position: relative;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 8px;
  background: rgb(var(--v-theme-surface));
  overflow: hidden;
}
/* CodeMirror 容器填满编辑区；深浅主题由编辑器扩展自行着色 */
.prompt-cm {
  min-height: 480px;
}
.prompt-cm :deep(.cm-editor) {
  border-radius: 8px;
}
.prompt-cm :deep(.cm-editor.cm-focused) {
  outline: none;
}
/* 降级纯文本编辑器 */
.prompt-fallback {
  display: block;
  width: 100%;
  min-height: 480px;
  padding: 12px;
  border: none;
  outline: none;
  resize: vertical;
  background: transparent;
  color: rgba(var(--v-theme-on-surface), var(--v-high-emphasis-opacity));
  font-family: 'Monaspace Neon', 'JetBrains Mono', Consolas, 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.7;
}
.prompt-fallback::placeholder {
  color: rgba(var(--v-theme-on-surface), 0.45);
}
</style>
