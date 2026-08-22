<script lang="ts">
/** 自定义右键菜单项；divider 项只渲染分隔线 */
export interface ContextMenuItem {
  label?: string
  icon?: string
  color?: string
  disabled?: boolean
  divider?: boolean
  action?: () => void
}
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted } from 'vue'

const props = defineProps<{
  items: ContextMenuItem[]
  x: number
  y: number
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

function pick(item: ContextMenuItem) {
  emit('close')
  item.action?.()
}

function onDocClick() {
  emit('close')
}

function onDocKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close')
}

function onWindowBlur() {
  emit('close')
}

onMounted(() => {
  document.addEventListener('click', onDocClick)
  document.addEventListener('keydown', onDocKeydown)
  window.addEventListener('blur', onWindowBlur)
})
onBeforeUnmount(() => {
  document.removeEventListener('click', onDocClick)
  document.removeEventListener('keydown', onDocKeydown)
  window.removeEventListener('blur', onWindowBlur)
})

/** 靠近屏幕边缘时向内收缩，避免菜单溢出窗口 */
const style = computed(() => ({
  left: Math.min(props.x, window.innerWidth - 190) + 'px',
  top: Math.min(props.y, window.innerHeight - 80) + 'px',
}))
</script>

<template>
  <div class="ctx-menu" :style="style" @contextmenu.prevent @click.stop>
    <v-list density="compact" nav>
      <template v-for="(item, i) in items" :key="i">
        <v-divider v-if="item.divider" />
        <v-list-item v-else :disabled="item.disabled" :color="item.color" @click="pick(item)">
          <template v-if="item.icon" #prepend>
            <v-icon :icon="item.icon" :color="item.color" />
          </template>
          <v-list-item-title>{{ item.label }}</v-list-item-title>
        </v-list-item>
      </template>
    </v-list>
  </div>
</template>

<style scoped>
.ctx-menu {
  position: fixed;
  z-index: 2200;
  min-width: 180px;
  padding: 4px 0;
  border: 1px solid rgba(var(--v-border-color), var(--v-border-opacity));
  border-radius: 6px;
  background: rgb(var(--v-theme-surface));
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.2);
  overflow: hidden;
}
</style>