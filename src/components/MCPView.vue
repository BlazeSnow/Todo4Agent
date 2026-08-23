<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { createHighlighter, type Highlighter } from 'shiki'

const props = defineProps<{
  /** 当前登录用户名 */
  currentUser: string | null
}>()

const emit = defineEmits<{
  (e: 'notify', msg: string): void
}>()

// MCP 工具清单（与后端 mcp.rs 保持一致）
const mcpTools = [
  'app_version / app_release',
  'group_list / group_create / group_rename / group_delete',
  'task_list / task_create / task_update',
  'task_complete / task_delete / task_export / task_import',
  'user_password',
  'prompt_get / prompt_update',
]

async function copyCommand() {
  try {
    await navigator.clipboard.writeText('todo4agent mcp')
    emit('notify', '已复制命令：todo4agent mcp')
  } catch {
    emit('notify', '复制失败，请手动复制')
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
            TODO4AGENT_USERNAME: props.currentUser ?? '你的用户名',
            TODO4AGENT_PASSWORD: '你的密码',
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
    emit('notify', '已复制 Agent 客户端配置')
  } catch {
    emit('notify', '复制失败，请手动复制')
  }
}
</script>

<template>
  <div>
    <h2 class="text-h6 mb-1">Agent 接入（MCP）</h2>
    <p class="text-body-2 text-medium-emphasis mb-4">通过 MCP 协议让 Agent 操作任务清单</p>

    <v-card variant="outlined" class="mb-4">
      <v-card-text>
        <p class="mb-3">
          本软件通过 MCP（Model Context Protocol，stdio 传输）向 Agent 暴露任务清单能力，
          Agent 以子进程方式启动并连接，与桌面端共用同一个数据库。
        </p>
        <p class="mb-3">
          通过环境变量指定用户名与密码进行身份验证，验证失败将拒绝启动。
        </p>

        <div class="d-flex align-center">
          <v-chip class="font-mono mr-2" variant="outlined" label>
            todo4agent mcp
          </v-chip>
          <v-btn size="small" variant="tonal" prepend-icon="mdi-content-copy" @click="copyCommand">
            复制命令
          </v-btn>
        </div>
      </v-card-text>
    </v-card>

    <v-card variant="outlined" class="mb-4">
      <v-card-title class="d-flex align-center justify-space-between gap-2">
        <span>Agent 客户端配置示例</span>
        <v-btn
          size="small"
          variant="tonal"
          prepend-icon="mdi-content-copy"
          @click="copyConfig"
        >
          复制配置
        </v-btn>
      </v-card-title>
      <v-card-text>
        <!-- Shiki 渲染的代码块（v-html 内容为本地生成的配置串，安全）；
             高亮不可用时回退纯文本 -->
        <div v-if="configHtml" class="code-block rounded" v-html="configHtml" />
        <pre v-else class="code-block pa-3 rounded"><code>{{ configText }}</code></pre>
      </v-card-text>
    </v-card>

    <v-card variant="outlined">
      <v-card-title>可用工具</v-card-title>
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
  font-family: 'JetBrains Mono', 'Fira Code', Consolas, 'Courier New', monospace;
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