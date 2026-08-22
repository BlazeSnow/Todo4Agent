<script setup lang="ts">
import type { Group } from '../types'

defineProps<{
  groups: Group[]
  selectedId: number | null
  loading: boolean
}>()

defineEmits<{
  (e: 'select', id: number): void
  (e: 'create'): void
  (e: 'rename', group: Group): void
  (e: 'delete', group: Group): void
  (e: 'mcp'): void
}>()
</script>

<template>
  <v-list nav density="comfortable">
    <v-list-subheader>任务分组</v-list-subheader>

    <v-list-item
      v-for="group in groups"
      :key="group.id"
      :active="selectedId === group.id"
      :title="group.name"
      @click="$emit('select', group.id)"
    >
      <template #prepend>
        <v-icon icon="mdi-folder" />
      </template>
      <template #append>
        <v-menu location="bottom right" :close-on-content-click="true">
          <template #activator="{ props }">
            <v-btn
              v-bind="props"
              icon="mdi-dots-horizontal"
              size="small"
              variant="text"
              @click.stop
            />
          </template>
          <v-list density="compact">
            <v-list-item
              prepend-icon="mdi-pencil"
              title="重命名"
              @click="$emit('rename', group)"
            />
            <v-list-item
              prepend-icon="mdi-delete"
              title="删除"
              color="error"
              @click="$emit('delete', group)"
            />
          </v-list>
        </v-menu>
      </template>
    </v-list-item>

    <v-list-item prepend-icon="mdi-plus" title="新增分组" @click="$emit('create')" />

    <v-divider class="my-2" />

    <v-list-subheader>Agent 接入</v-list-subheader>
    <v-list-item
      prepend-icon="mdi-connection"
      title="Agent 接入（MCP）"
      subtitle="点击查看连接说明"
      @click="$emit('mcp')"
    />
  </v-list>
</template>