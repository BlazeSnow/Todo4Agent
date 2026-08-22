<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import {
  authChangePassword,
  authRegister,
  downloadExport,
  exportDoc,
  getSettings,
  importDoc,
  updateSettings,
} from '../api'
import type { ExportDoc } from '../types'
import packageJson from '../../package.json'

const props = defineProps<{
  /** 当前登录用户名（本地模式为 null） */
  currentUser: string | null
}>()

const emit = defineEmits<{
  (e: 'exported'): void
  (e: 'imported'): void
  (e: 'logout'): void
  (e: 'authChanged'): void
  (e: 'error', msg: string): void
  (e: 'notify', msg: string): void
}>()

const exporting = ref(false)
const importing = ref(false)

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

/** 选择本地 JSON 文件并导入 */
function onPickImport() {
  const input = document.createElement('input')
  input.type = 'file'
  input.accept = '.json,application/json'
  input.onchange = async () => {
    const file = input.files?.[0]
    if (!file) return
    let doc: ExportDoc
    try {
      doc = JSON.parse(await file.text())
      if (!Array.isArray(doc.groups)) throw new Error('缺少 groups 字段')
    } catch (e) {
      emit('error', '文件不是有效的 Todo4Agent 导出 JSON')
      return
    }
    importing.value = true
    try {
      const r = await importDoc(doc)
      emit('imported')
      emit(
        'notify',
        `导入完成：新建 ${r.groups_created} 组、并入 ${r.groups_merged} 组、导入 ${r.tasks_imported} 个任务${r.tasks_skipped ? `、跳过 ${r.tasks_skipped} 个空任务` : ''}`,
      )
    } catch (e) {
      emit('error', (e as Error).message)
    } finally {
      importing.value = false
    }
  }
  input.click()
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

// ---------- 用户 ----------

const newUser = ref('')
const newPass = ref('')
const registering = ref(false)

async function registerUser() {
  const name = newUser.value.trim()
  if (!name || newPass.value.length < 4) return
  registering.value = true
  try {
    await authRegister(name, newPass.value)
    emit('notify', `用户「${name}」已创建`)
    // 系统可能刚进入多用户模式（首个用户），重新校验认证状态
    emit('authChanged')
    newUser.value = ''
    newPass.value = ''
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    registering.value = false
  }
}

const oldPass = ref('')
const newPass2 = ref('')
const changingPass = ref(false)

async function changePassword() {
  if (oldPass.value.length < 4 || newPass2.value.length < 4) return
  changingPass.value = true
  try {
    await authChangePassword(oldPass.value, newPass2.value)
    emit('notify', '密码已修改')
    oldPass.value = ''
    newPass2.value = ''
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    changingPass.value = false
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
      <v-card-title>用户</v-card-title>
      <v-card-text>
        <div class="d-flex align-center mb-3">
          <v-icon icon="mdi-account-circle" class="mr-2" color="primary" />
          <span class="text-body-1">当前用户：{{ currentUser ?? '未登录（本地模式）' }}</span>
          <v-spacer />
          <v-btn
            v-if="currentUser"
            variant="tonal"
            prepend-icon="mdi-logout"
            @click="$emit('logout')"
          >
            退出登录
          </v-btn>
        </div>

        <v-divider class="my-3" />

        <div class="text-subtitle-2 mb-2">注册新用户</div>
        <v-text-field
          v-model="newUser"
          label="用户名"
          class="mb-2"
          hide-details="auto"
        />
        <v-text-field
          v-model="newPass"
          label="密码（至少 4 位）"
          type="password"
          class="mb-2"
          hide-details="auto"
        />
        <v-btn
          color="primary"
          variant="tonal"
          :loading="registering"
          :disabled="!newUser.trim() || newPass.length < 4"
          @click="registerUser"
        >
          创建用户
        </v-btn>
        <p class="text-caption mt-2 text-medium-emphasis">
          首个用户将接管当前本地数据；之后注册的用户拥有独立数据空间。
        </p>

        <v-divider class="my-3" />

        <div class="text-subtitle-2 mb-2">修改密码</div>
        <v-text-field
          v-model="oldPass"
          label="原密码"
          type="password"
          class="mb-2"
          hide-details="auto"
        />
        <v-text-field
          v-model="newPass2"
          label="新密码（至少 4 位）"
          type="password"
          class="mb-2"
          hide-details="auto"
        />
        <v-btn
          variant="tonal"
          :loading="changingPass"
          :disabled="oldPass.length < 4 || newPass2.length < 4"
          @click="changePassword"
        >
          修改密码
        </v-btn>
      </v-card-text>
    </v-card>

    <v-card variant="outlined" class="mb-4">
      <v-card-title>数据</v-card-title>
      <v-card-text>
        <div class="d-flex align-center ga-3">
          <v-btn
            color="primary"
            prepend-icon="mdi-export-variant"
            :loading="exporting"
            @click="onExport"
          >
            导出 JSON
          </v-btn>
          <v-btn
            variant="tonal"
            prepend-icon="mdi-import"
            :loading="importing"
            @click="onPickImport"
          >
            导入 JSON
          </v-btn>
        </div>
        <p class="text-caption mt-2 text-medium-emphasis">
          导出全部任务清单为 JSON 文件，便于备份或迁移；导入时同名分组会并入（任务追加），新分组新建。
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