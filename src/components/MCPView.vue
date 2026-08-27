<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { createHighlighter, type Highlighter } from 'shiki'
import InfoTip from './InfoTip.vue'

const props = defineProps<{
  /** 当前登录用户名 */
  currentUser: string | null
}>()

const emit = defineEmits<{
  (e: 'notify', msg: string): void
}>()

const { t } = useI18n()

// MCP 工具清单（与后端 mcp.rs 保持一致）
const mcpTools = [
  'app_version / app_release / db_path',
  'group_list / group_create / group_rename / group_delete',
  'task_list / task_create / task_update',
  'task_complete / task_archive / task_unarchive',
  'task_delete / task_export / task_import',
  'user_password',
  'prompt_get / prompt_update',
]

async function copyCommand() {
  try {
    await navigator.clipboard.writeText('todo4agent mcp')
    emit('notify', t('mcp.commandCopied'))
  } catch {
    emit('notify', t('mcp.copyFailed'))
  }
}

/** 客户端配置示例：用户名动态填入当前登录用户 */
const configText = computed(() =>
  JSON.stringify(
    {
      mcpServers: {
        todo4agent: {
          command: 'todo4agent',
          args: ['mcp'],
          env: {
            TODO4AGENT_USERNAME: props.currentUser ?? t('mcp.usernamePlaceholder'),
            TODO4AGENT_PASSWORD: t('mcp.passwordPlaceholder'),
          },
        },
      },
    },
    null,
    2,
  ),
)

// ---------- 配置示例高亮（Shiki 双主题） ----------

/** Shiki 输出的 HTML：内嵌 light/dark 两组 CSS 变量，随 Vuetify 深浅色切换 */
const configHtml = ref('')
let highlighter: Highlighter | null = null

function renderHighlighted() {
  if (!highlighter) return
  configHtml.value = highlighter.codeToHtml(configText.value, {
    lang: 'json',
    themes: { light: 'github-light', dark: 'github-dark' },
    // 只输出 light/dark CSS 变量，不把浅色字面值写进内联 style，
    // 否则内联样式优先级高于主题切换 CSS，深色模式下仍是浅色
    defaultColor: false,
  })
}

onMounted(async () => {
  try {
    highlighter = await createHighlighter({
      themes: ['github-light', 'github-dark'],
      langs: ['json'],
    })
    renderHighlighted()
  } catch {
    // 高亮引擎加载失败时回退纯文本代码块
    configHtml.value = ''
  }
})

watch(configText, renderHighlighted)

async function copyConfig() {
  try {
    await navigator.clipboard.writeText(configText.value)
    emit('notify', t('mcp.configCopied'))
  } catch {
    emit('notify', t('mcp.copyFailed'))
  }
}
</script>

<template>
  <div>
    <h2 class="text-h6 mb-1">{{ t('mcp.title') }}</h2>
    <p class="text-body-2 text-medium-emphasis mb-4">{{ t('mcp.subtitle') }}</p>

    <v-card class="mb-4">
      <v-card-title class="d-flex align-center">
        {{ t('mcp.launchTitle') }}
        <InfoTip>
          <p class="mb-2">{{ t('mcp.intro1') }}</p>
          <p class="mb-0">{{ t('mcp.intro2') }}</p>
        </InfoTip>
      </v-card-title>
      <v-card-text>
        <div class="d-flex align-center">
          <v-chip class="font-mono mr-2" variant="outlined" label>
            todo4agent mcp
          </v-chip>
          <v-btn size="small" variant="tonal" prepend-icon="mdi-content-copy" @click="copyCommand">
            {{ t('mcp.copyCommand') }}
          </v-btn>
        </div>
      </v-card-text>
    </v-card>

    <v-card class="mb-4">
      <v-card-title class="d-flex align-center justify-space-between gap-2">
        <span>{{ t('mcp.configTitle') }}</span>
        <v-btn
          size="small"
          variant="tonal"
          prepend-icon="mdi-content-copy"
          @click="copyConfig"
        >
          {{ t('mcp.copyConfig') }}
        </v-btn>
      </v-card-title>
      <v-card-text>
        <!-- Shiki 渲染的代码块（v-html 内容为本地生成的配置串，安全）；
             高亮不可用时回退纯文本 -->
        <div v-if="configHtml" class="code-block rounded" v-html="configHtml" />
        <pre v-else class="code-block pa-3 rounded"><code>{{ configText }}</code></pre>
      </v-card-text>
    </v-card>

    <v-card>
      <v-card-title>{{ t('mcp.tools') }}</v-card-title>
      <v-card-text>
        <v-list density="compact">
          <v-list-item v-for="t in mcpTools" :key="t">
            <template #prepend>
              <v-icon icon="mdi-wrench" size="small" class="mr-2" />
            </template>
            <v-list-item-title class="font-mono text-body-2">{{ t }}</v-list-item-title>
          </v-list-item>
        </v-list>
      </v-card-text>
    </v-card>
  </div>
</template>

<style>
/* Shiki 双主题：按 Vuetify 主题类切换内嵌的 light/dark CSS 变量 */
.shiki,
.shiki span {
  color: var(--shiki-light);
  font-style: var(--shiki-light-font-style);
  font-weight: var(--shiki-light-font-weight);
  text-decoration: var(--shiki-light-text-decoration);
}
.shiki {
  background-color: var(--shiki-light-bg);
}
.v-theme--dark .shiki,
.v-theme--dark .shiki span {
  color: var(--shiki-dark);
  font-style: var(--shiki-dark-font-style);
  font-weight: var(--shiki-dark-font-weight);
  text-decoration: var(--shiki-dark-text-decoration);
}
.v-theme--dark .shiki {
  background-color: var(--shiki-dark-bg);
}

/* Shiki 代码块排版（含回退的纯文本代码块） */
.code-block pre {
  margin: 0;
  padding: 12px;
  overflow-x: auto;
  font-size: 13px;
  line-height: 1.6;
}

/* 代码字体：Shiki 高亮块（.shiki code）与高亮不可用时的纯文本回退（pre.code-block） */
.shiki code,
pre.code-block {
  font-family: 'Monaspace Neon', 'JetBrains Mono', Consolas, 'Courier New', monospace;
}

/* 高亮不可用时的纯文本回退：深色主题用近黑面板色，浅色沿用 surface-variant */
.v-theme--dark pre.code-block {
  background: #16181d;
  color: #d6d9df;
}
.v-theme--light pre.code-block {
  background: rgb(var(--v-theme-surface-variant));
}
</style>