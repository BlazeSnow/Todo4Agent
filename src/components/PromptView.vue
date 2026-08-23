<script setup lang="ts">
import { onMounted, ref } from 'vue'
import ConfirmDialog from './ConfirmDialog.vue'
import { getPrompt, savePrompt } from '../api'

const emit = defineEmits<{
  (e: 'notify', msg: string): void
  (e: 'error', msg: string): void
}>()

const content = ref('')
const isDefault = ref(true)
const updatedAt = ref<string | null>(null)
const loading = ref(false)
const saving = ref(false)
const confirmClear = ref(false)

onMounted(async () => {
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

async function save() {
  saving.value = true
  try {
    const p = await savePrompt(content.value)
    content.value = p.content
    isDefault.value = p.is_default
    updatedAt.value = p.updated_at
    emit('notify', isDefault.value ? '已清空提示词' : '提示词已保存')
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    saving.value = false
  }
}

async function copy() {
  if (!content.value) {
    emit('notify', '提示词为空，先填写内容再复制')
    return
  }
  try {
    await navigator.clipboard.writeText(content.value)
    emit('notify', '已复制提示词')
  } catch {
    emit('notify', '复制失败，请手动复制')
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
  return isNaN(d.getTime()) ? iso : d.toLocaleString('zh-CN', { dateStyle: 'short', timeStyle: 'short' })
}
</script>

<template>
  <div>
    <div class="d-flex align-center mb-1">
      <h2 class="text-h6">提示词</h2>
      <v-spacer />
      <v-btn variant="text" prepend-icon="mdi-content-copy" :disabled="!content" @click="copy">复制</v-btn>
      <v-btn
        variant="text"
        color="error"
        prepend-icon="mdi-eraser"
        :disabled="isDefault || loading"
        @click="confirmClear = true"
      >
        清空
      </v-btn>
    </div>
    <p class="text-body-2 text-medium-emphasis mb-4">
      给 Agent 的协作规范（类似 AGENTS.md）：复制到 Agent 客户端的系统提示词使用，
      也可让 Agent 通过 MCP 的 <span class="font-mono">prompt_get</span> /
      <span class="font-mono">prompt_update</span> 工具直接读写这份提示词。
      默认为空，内容由你自行发挥。
    </p>

    <v-textarea
      v-model="content"
      :loading="loading"
      :disabled="loading"
      auto-grow
      rows="18"
      hide-details
      placeholder="填写给 Agent 的协作规范，例如任务书写习惯、分组约定、汇报方式等；留空表示暂不使用提示词"
      class="prompt-editor mb-3"
    />

    <div class="d-flex align-center">
      <v-btn
        color="primary"
        prepend-icon="mdi-content-save"
        :loading="saving"
        :disabled="loading"
        @click="save"
      >
        保存
      </v-btn>
      <span class="text-caption text-medium-emphasis ml-3">
        {{ isDefault ? '未设置（默认为空）' : `已自定义 · 更新于 ${formatTime(updatedAt)}` }}
      </span>
    </div>

    <ConfirmDialog
      v-model="confirmClear"
      message="将清空提示词并回到默认空状态。确定继续吗？"
      @confirm="clearAll"
    />
  </div>
</template>

<style scoped>
.prompt-editor :deep(textarea) {
  font-family: 'JetBrains Mono', 'Fira Code', Consolas, 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.7;
}
</style>
