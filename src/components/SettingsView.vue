<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { downloadExport, exportDoc, getSettings, updateSettings } from '../api'
import packageJson from '../../package.json'

const emit = defineEmits<{
  (e: 'exported'): void
  (e: 'error', msg: string): void
  (e: 'notify', msg: string): void
}>()

const exporting = ref(false)

async function onExport() {
  exporting.value = true
  try {
    const doc = await exportDoc()
    downloadExport(doc)
    emit('exported')
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    exporting.value = false
  }
}

// ---------- 服务端口 ----------

const effectivePort = ref<number | null>(null)
const portInput = ref('3000')
const savingPort = ref(false)

const portValid = computed(() => {
  const n = Number(portInput.value)
  return Number.isInteger(n) && n >= 1024 && n <= 65535
})

onMounted(async () => {
  try {
    const s = await getSettings()
    effectivePort.value = s.effective_port
    portInput.value = String(s.port)
  } catch (e) {
    emit('error', (e as Error).message)
  }
})

async function savePort() {
  if (!portValid.value) return
  savingPort.value = true
  try {
    await updateSettings(Number(portInput.value))
    emit('notify', '端口已保存，重启应用后生效')
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    savingPort.value = false
  }
}
</script>

<template>
  <div>
    <h2 class="text-h6 mb-1">设置</h2>
    <p class="text-body-2 text-medium-emphasis mb-4">数据、服务与关于信息</p>

    <v-card variant="outlined" class="mb-4">
      <v-card-title>服务</v-card-title>
      <v-card-text>
        <div v-if="effectivePort != null" class="text-body-2 mb-3">
          当前监听端口：<span class="font-mono">{{ effectivePort }}</span>
        </div>
        <v-text-field
          v-model="portInput"
          label="WebUI / API 端口（1024-65535）"
          type="number"
          :rules="[() => portValid || '端口范围：1024-65535']"
          hide-details="auto"
          class="mb-2"
        />
        <div class="d-flex align-center">
          <v-btn color="primary" :loading="savingPort" :disabled="!portValid" @click="savePort">
            保存
          </v-btn>
          <span class="text-caption text-medium-emphasis ml-3">
            修改后需重启应用生效
          </span>
        </div>
      </v-card-text>
    </v-card>

    <v-card variant="outlined" class="mb-4">
      <v-card-title>数据</v-card-title>
      <v-card-text>
        <v-btn color="primary" prepend-icon="mdi-export-variant" :loading="exporting" @click="onExport">
          导出 JSON
        </v-btn>
        <p class="text-caption mt-2 text-medium-emphasis">
          将全部任务清单导出为 JSON 文件，便于备份或迁移。
        </p>
      </v-card-text>
    </v-card>

    <v-card variant="outlined">
      <v-card-title>关于</v-card-title>
      <v-card-text>
        <v-list density="compact">
          <v-list-item title="版本">
            <template #append>
              <span class="text-medium-emphasis">v{{ packageJson.version }}</span>
            </template>
          </v-list-item>
          <v-list-item title="软件说明">
            <template #append>
              <span class="text-medium-emphasis text-right">为 Agent 设计的 MCP 任务清单</span>
            </template>
          </v-list-item>
          <v-list-item title="仓库">
            <template #append>
              <span class="text-medium-emphasis">github.com/BlazeSnow/Todo4Agent</span>
            </template>
          </v-list-item>
        </v-list>
      </v-card-text>
    </v-card>
  </div>
</template>