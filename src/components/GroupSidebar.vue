<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n()

/** 分组显示名：系统分组「无分组」按界面语言显示，其余为存储名 */
const displayName = (g: Group) => (g.name === NO_GROUP_NAME ? t('groups.noGroup') : g.name)

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
        label: t('common.moveUp'),
        icon: 'mdi-arrow-up',
        disabled: !canMoveGroup(group, -1),
        action: () => moveGroup(group, -1),
      },
      {
        label: t('common.moveDown'),
        icon: 'mdi-arrow-down',
        disabled: !canMoveGroup(group, 1),
        action: () => moveGroup(group, 1),
      },
      { divider: true },
      // 系统分组「无分组」承载被删分组的任务，不可编辑/删除，也不可锁定（兜底去处须始终可编辑）
      ...(group.name === NO_GROUP_NAME
        ? []
        : [
            {
              label: t('sidebar.editGroup'),
              icon: 'mdi-pencil',
              action: () => emit('rename', group),
            },
          ]),
      ...(group.name === NO_GROUP_NAME
        ? []
        : [
            {
              label: group.locked ? t('sidebar.unlock') : t('sidebar.lock'),
              icon: group.locked ? 'mdi-lock-open' : 'mdi-lock',
              action: () => emit('toggle-lock', group),
            },
          ]),
      ...(group.name === NO_GROUP_NAME
        ? []
        : [
            {
              label: t('common.delete'),
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
    <v-list-subheader>{{ t('sidebar.groups') }}</v-list-subheader>

    <v-list-item
      v-for="group in groups"
      :key="group.id"
      :active="activeView === 'tasks' && selectedId === group.id"
      :title="displayName(group)"
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
              :title="t('common.moveUp')"
              :disabled="!canMoveGroup(group, -1)"
              @click="moveGroup(group, -1)"
            />
            <v-list-item
              prepend-icon="mdi-arrow-down"
              :title="t('common.moveDown')"
              :disabled="!canMoveGroup(group, 1)"
              @click="moveGroup(group, 1)"
            />
            <v-divider />
            <v-list-item
              v-if="group.name !== NO_GROUP_NAME"
              prepend-icon="mdi-pencil"
              :title="t('sidebar.editGroup')"
              :subtitle="t('sidebar.editGroupSubtitle')"
              @click="$emit('rename', group)"
            />
            <v-list-item
              v-if="group.name !== NO_GROUP_NAME"
              :prepend-icon="group.locked ? 'mdi-lock-open' : 'mdi-lock'"
              :title="group.locked ? t('sidebar.unlock') : t('sidebar.lock')"
              :subtitle="group.locked ? t('sidebar.lockedHint') : t('sidebar.lockHint')"
              @click="$emit('toggle-lock', group)"
            />
            <v-list-item
              v-if="group.name !== NO_GROUP_NAME"
              prepend-icon="mdi-delete"
              :title="t('common.delete')"
              color="error"
              @click="$emit('delete', group)"
            />
          </v-list>
        </v-menu>
      </template>
    </v-list-item>

    <v-list-item prepend-icon="mdi-plus" :title="t('sidebar.addGroup')" @click="$emit('create')" />

    <v-list-item
      prepend-icon="mdi-archive-outline"
      :title="t('sidebar.archive')"
      :active="activeView === 'archive'"
      @click="$emit('archive')"
    />

    <v-list-item
      prepend-icon="mdi-trash-can-outline"
      :title="t('sidebar.trash')"
      :active="activeView === 'trash'"
      @click="$emit('trash')"
    />

    <v-divider class="my-2" />

    <v-list-subheader>{{ t('sidebar.more') }}</v-list-subheader>
    <v-list-item
      prepend-icon="mdi-connection"
      :title="t('sidebar.mcp')"
      :subtitle="t('sidebar.mcpSubtitle')"
      :active="activeView === 'mcp'"
      @click="$emit('mcp')"
    />

    <v-list-item
      prepend-icon="mdi-script-text-outline"
      :title="t('sidebar.prompt')"
      :subtitle="t('sidebar.promptSubtitle')"
      :active="activeView === 'prompt'"
      @click="$emit('prompt')"
    />

    <v-list-item
      prepend-icon="mdi-cog-outline"
      :title="t('sidebar.settings')"
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