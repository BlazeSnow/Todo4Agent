<script setup lang="ts">
import { computed } from 'vue'

const props = defineProps<{
  /** 当前登录用户名（本地模式为 null） */
  currentUser: string | null
}>()

const emit = defineEmits<{
  (e: 'notify', msg: string): void
}>()

// MCP 工具清单（与后端 mcp.rs 保持一致）
const mcpTools = [
  'app_version',
  'group_list / group_create / group_rename',
  'task_list / task_create / task_update',
  'task_complete / task_delete / task_export',
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
      <v-card-title>
        Agent 客户端配置示例
        <template #append>
          <v-btn
            size="small"
            variant="tonal"
            prepend-icon="mdi-content-copy"
            @click="copyConfig"
          >
            复制配置
          </v-btn>
        </template>
      </v-card-title>
      <v-card-text>
        <pre class="bg-surface-variant pa-3 rounded"><code>{{ configText }}</code></pre>
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