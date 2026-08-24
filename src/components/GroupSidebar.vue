<script setup lang="ts">
import { ref } from 'vue'
import { NO_GROUP_NAME, type Group } from '../types'
import ContextMenu, { type ContextMenuItem } from './ContextMenu.vue'

const props = defineProps<{
  groups: Group[]
  selectedId: number | null
  loading: boolean
  /** 当前主视图，用于侧边栏底部入口的选中高亮 */
  activeView: 'tasks' | 'settings' | 'mcp' | 'prompt' | 'archive' | 'trash'
}>()

const emit = defineEmits<{
  (e: 'select', id: number): void
  (e: 'create'): void
  (e: 'rename', group: Group): void
  (e: 'delete', group: Group): void
  (e: 'toggle-lock', group: Group): void
  (e: 'mcp'): void
  (e: 'prompt'): void
  (e: 'settings'): void
  (e: 'archive'): void
  (e: 'trash'): void
  (e: 'reorder', groupIds: number[]): void
}>()

// ---------- 上移 / 下移 ----------

function canMoveGroup(group: Group, dir: -1 | 1): boolean {
  const idx = props.groups.findIndex((g) => g.id === group.id)
  if (idx < 0) return false
  const target = idx + dir
  return target >= 0 && target < props.groups.length
}

function moveGroup(group: Group, dir: -1 | 1) {
  if (!canMoveGroup(group, dir)) return
  const list = [...props.groups]
  const idx = list.findIndex((g) => g.id === group.id)
  const target = idx + dir
  const tmp = list[idx]
  list[idx] = list[target]
  list[target] = tmp
  emit('reorder', list.map((g) => g.id))
}

// ---------- 右键菜单 ----------

const groupCtx = ref<{ x: number; y: number; items: ContextMenuItem[] } | null>(null)

function openGroupCtx(group: Group, e: MouseEvent) {
  e.preventDefault()
  groupCtx.value = {
    x: e.clientX,
    y: e.clientY,
    items: [
      {
        label: '上移',
        icon: 'mdi-arrow-up',
        disabled: !canMoveGroup(group, -1),
        action: () => moveGroup(group, -1),
      },
      {
        label: '下移',
        icon: 'mdi-arrow-down',
        disabled: !canMoveGroup(group, 1),
        action: () => moveGroup(group, 1),
      },
      { divider: true },
      // 系统分组「无分组」承载被删分组的任务，不可编辑/删除，也不可锁定（兜底去处须始终可编辑）
      ...(group.name === NO_GROUP_NAME
        ? []
        : [{ label: '编辑分组', icon: 'mdi-pencil', action: () => emit('rename', group) }]),
      ...(group.name === NO_GROUP_NAME
        ? []
        : [
            {
              label: group.locked ? '解锁清单' : '锁定清单',
              icon: group.locked ? 'mdi-lock-open' : 'mdi-lock',
              action: () => emit('toggle-lock', group),
            },
          ]),
      ...(group.name === NO_GROUP_NAME
        ? []
        : [
            {
              label: '删除',
              icon: 'mdi-delete',
              color: 'error',
              action: () => emit('delete', group),
            },
          ]),
    ],
  }
}
</script>

<template>
  <v-list nav density="comfortable">
    <v-list-subheader>任务分组</v-list-subheader>

    <v-list-item
      v-for="group in groups"
      :key="group.id"
      :active="activeView === 'tasks' && selectedId === group.id"
      :title="group.name"
      @click="$emit('select', group.id)"
      @contextmenu.stop="openGroupCtx(group, $event)"
    >
      <template #prepend>
        <v-icon :icon="group.locked ? 'mdi-folder-lock' : 'mdi-folder'" />
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
              prepend-icon="mdi-arrow-up"
              title="上移"
              :disabled="!canMoveGroup(group, -1)"
              @click="moveGroup(group, -1)"
            />
            <v-list-item
              prepend-icon="mdi-arrow-down"
              title="下移"
              :disabled="!canMoveGroup(group, 1)"
              @click="moveGroup(group, 1)"
            />
            <v-divider />
            <v-list-item
              v-if="group.name !== NO_GROUP_NAME"
              prepend-icon="mdi-pencil"
              title="编辑分组"
              subtitle="名称与描述"
              @click="$emit('rename', group)"
            />
            <v-list-item
              v-if="group.name !== NO_GROUP_NAME"
              :prepend-icon="group.locked ? 'mdi-lock-open' : 'mdi-lock'"
              :title="group.locked ? '解锁清单' : '锁定清单'"
              :subtitle="group.locked ? 'Agent 当前无法编辑' : '锁定后仅自己可编辑'"
              @click="$emit('toggle-lock', group)"
            />
            <v-list-item
              v-if="group.name !== NO_GROUP_NAME"
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

    <v-list-item
      prepend-icon="mdi-archive-outline"
      title="归档"
      :active="activeView === 'archive'"
      @click="$emit('archive')"
    />

    <v-list-item
      prepend-icon="mdi-trash-can-outline"
      title="回收站"
      :active="activeView === 'trash'"
      @click="$emit('trash')"
    />

    <v-divider class="my-2" />

    <v-list-subheader>更多</v-list-subheader>
    <v-list-item
      prepend-icon="mdi-connection"
      title="Agent 接入（MCP）"
      subtitle="点击查看连接说明"
      :active="activeView === 'mcp'"
      @click="$emit('mcp')"
    />

    <v-list-item
      prepend-icon="mdi-script-text-outline"
      title="提示词"
      subtitle="Agent 协作规范"
      :active="activeView === 'prompt'"
      @click="$emit('prompt')"
    />

    <v-list-item
      prepend-icon="mdi-cog-outline"
      title="设置"
      :active="activeView === 'settings'"
      @click="$emit('settings')"
    />
  </v-list>

  <ContextMenu
    v-if="groupCtx"
    :items="groupCtx.items"
    :x="groupCtx.x"
    :y="groupCtx.y"
    @close="groupCtx = null"
  />
</template>