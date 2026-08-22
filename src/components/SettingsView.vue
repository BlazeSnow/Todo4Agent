<script setup lang="ts">
import { ref } from 'vue'
import { downloadExport, exportDoc } from '../api'
import packageJson from '../../package.json'

const emit = defineEmits<{
  (e: 'exported'): void
  (e: 'error', msg: string): void
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
</script>

<template>
  <div>
    <h2 class="text-h6 mb-1">设置</h2>
    <p class="text-body-2 text-medium-emphasis mb-4">数据与关于信息</p>

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