<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
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

    <v-textarea
      v-model="content"
      :loading="loading"
      :disabled="loading"
      auto-grow
      rows="18"
      hide-details
      :placeholder="t('prompt.placeholder')"
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
.prompt-editor :deep(textarea) {
  font-family: 'Monaspace Neon', 'JetBrains Mono', Consolas, 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.7;
}
</style>
