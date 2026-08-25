<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  authChangePassword,
  downloadExport,
  exportDoc,
  getSettings,
  importDoc,
  openDbLocation,
  updateSettings,
} from '../api'
import type { ExportDoc } from '../types'
import packageJson from '../../package.json'

const props = defineProps<{
  /** 当前登录用户名 */
  currentUser: string | null
}>()

const emit = defineEmits<{
  (e: 'exported'): void
  (e: 'imported'): void
  (e: 'logout'): void
  (e: 'error', msg: string): void
  (e: 'notify', msg: string): void
}>()

const { t } = useI18n()

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
      if (!Array.isArray(doc.groups)) throw new Error('missing groups')
    } catch (e) {
      emit('error', t('settings.invalidFile'))
      return
    }
    importing.value = true
    try {
      const r = await importDoc(doc)
      emit('imported')
      emit(
        'notify',
        t('settings.importDone', {
          created: r.groups_created,
          merged: r.groups_merged,
          imported: r.tasks_imported,
        }) +
          (r.prompt_imported ? t('settings.importPrompt') : '') +
          (r.tasks_skipped ? t('settings.importSkipped', { skipped: r.tasks_skipped }) : ''),
      )
    } catch (e) {
      emit('error', (e as Error).message)
    } finally {
      importing.value = false
    }
  }
  input.click()
}

// ---------- 服务开关 ----------

const webuiLan = ref(true)
const allowRegister = ref(true)
const savingWebui = ref(false)
const savingRegister = ref(false)

/** 对外访问开关：立即保存，重启应用后对监听地址生效 */
async function onWebuiChange() {
  savingWebui.value = true
  try {
    await updateSettings({ webui_lan: webuiLan.value })
    emit('notify', webuiLan.value ? t('settings.webuiLanOn') : t('settings.webuiLanOff'))
  } catch (e) {
    webuiLan.value = !webuiLan.value
    emit('error', (e as Error).message)
  } finally {
    savingWebui.value = false
  }
}

/** 允许注册开关：立即生效 */
async function onRegisterChange() {
  savingRegister.value = true
  try {
    await updateSettings({ allow_register: allowRegister.value })
    emit('notify', allowRegister.value ? t('settings.registerOn') : t('settings.registerOff'))
  } catch (e) {
    allowRegister.value = !allowRegister.value
    emit('error', (e as Error).message)
  } finally {
    savingRegister.value = false
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
    webuiLan.value = s.webui_lan
    allowRegister.value = s.allow_register
    dbPath.value = s.db_path
  } catch (e) {
    emit('error', (e as Error).message)
  }
})

async function savePort() {
  if (!portValid.value) return
  savingPort.value = true
  try {
    await updateSettings({ port: Number(portInput.value) })
    emit('notify', t('settings.portSaved'))
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    savingPort.value = false
  }
}

// ---------- 数据库文件 ----------

const dbPath = ref('')
const openingDb = ref(false)

/** 在系统文件管理器中定位数据库文件（后端执行） */
async function onOpenDbLocation() {
  openingDb.value = true
  try {
    const r = await openDbLocation()
    dbPath.value = r.path
    emit('notify', t('settings.dbLocated'))
  } catch (e) {
    emit('error', (e as Error).message)
  } finally {
    openingDb.value = false
  }
}

// ---------- 用户 ----------

const oldPass = ref('')
const newPass2 = ref('')
const changingPass = ref(false)

async function changePassword() {
  if (oldPass.value.length < 4 || newPass2.value.length < 4) return
  changingPass.value = true
  try {
    await authChangePassword(oldPass.value, newPass2.value)
    emit('notify', t('settings.passwordChanged'))
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
    <h2 class="text-h6 mb-1">{{ t('settings.title') }}</h2>
    <p class="text-body-2 text-medium-emphasis mb-4">{{ t('settings.subtitle') }}</p>

    <v-card class="mb-4">
      <v-card-title>{{ t('settings.service') }}</v-card-title>
      <v-card-text>
        <v-switch
          v-model="webuiLan"
          color="primary"
          :label="t('settings.webuiLan')"
          :loading="savingWebui"
          hide-details
          class="mb-1"
          @change="onWebuiChange"
        />
        <p class="text-caption text-medium-emphasis mb-3">
          {{ t('settings.webuiLanHint') }}
        </p>
        <div v-if="effectivePort != null" class="text-body-2 mb-3">
          {{ t('settings.currentPort') }}<span class="font-mono">{{ effectivePort }}</span>
        </div>
        <v-text-field
          v-model="portInput"
          :label="t('settings.port')"
          type="number"
          :rules="[() => portValid || t('settings.portRange')]"
          hide-details="auto"
          class="mb-2"
        />
        <div class="d-flex align-center">
          <v-btn color="primary" :loading="savingPort" :disabled="!portValid" @click="savePort">
            {{ t('common.save') }}
          </v-btn>
          <span class="text-caption text-medium-emphasis ml-3">
            {{ t('settings.restartHint') }}
          </span>
        </div>
      </v-card-text>
    </v-card>

    <v-card class="mb-4">
      <v-card-title>{{ t('settings.user') }}</v-card-title>
      <v-card-text>
        <v-switch
          v-model="allowRegister"
          color="primary"
          :label="t('settings.allowRegister')"
          :loading="savingRegister"
          hide-details
          class="mb-1"
          @change="onRegisterChange"
        />
        <p class="text-caption text-medium-emphasis mb-3">
          {{ t('settings.allowRegisterHint') }}
        </p>
        <div class="d-flex align-center mb-3">
          <v-icon icon="mdi-account-circle" class="mr-2" color="primary" />
          <span class="text-body-1">{{ currentUser ? t('settings.currentUser', { name: currentUser }) : t('settings.notLoggedIn') }}</span>
          <v-spacer />
          <v-btn
            v-if="currentUser"
            variant="tonal"
            prepend-icon="mdi-logout"
            @click="$emit('logout')"
          >
            {{ t('settings.logout') }}
          </v-btn>
        </div>

        <v-divider class="my-3" />

        <div class="text-subtitle-2 mb-2">{{ t('settings.changePassword') }}</div>
        <form @submit.prevent="changePassword">
          <v-text-field
            v-model="oldPass"
            :label="t('settings.oldPassword')"
            type="password"
            autocomplete="current-password"
            class="mb-2"
            hide-details="auto"
          />
          <v-text-field
            v-model="newPass2"
            :label="t('settings.newPassword')"
            type="password"
            autocomplete="new-password"
            class="mb-2"
            hide-details="auto"
          />
          <v-btn
            type="submit"
            variant="tonal"
            :loading="changingPass"
            :disabled="oldPass.length < 4 || newPass2.length < 4"
          >
            {{ t('settings.changePassword') }}
          </v-btn>
        </form>
      </v-card-text>
    </v-card>

    <v-card class="mb-4">
      <v-card-title>{{ t('settings.data') }}</v-card-title>
      <v-card-text>
        <div v-if="dbPath" class="d-flex align-center ga-3 mb-4">
          <div class="text-body-2 flex-grow-1 db-path">
            {{ t('settings.dbFile') }}<span class="font-mono text-medium-emphasis">{{ dbPath }}</span>
          </div>
          <v-btn
            variant="tonal"
            prepend-icon="mdi-folder-open-outline"
            :loading="openingDb"
            @click="onOpenDbLocation"
          >
            {{ t('settings.openDbLocation') }}
          </v-btn>
        </div>
        <div class="d-flex align-center ga-3">
          <v-btn
            color="primary"
            prepend-icon="mdi-export-variant"
            :loading="exporting"
            @click="onExport"
          >
            {{ t('settings.exportJson') }}
          </v-btn>
          <v-btn
            variant="tonal"
            prepend-icon="mdi-import"
            :loading="importing"
            @click="onPickImport"
          >
            {{ t('settings.importJson') }}
          </v-btn>
        </div>
        <p class="text-caption mt-2 text-medium-emphasis">
          {{ t('settings.dataHint') }}
        </p>
      </v-card-text>
    </v-card>

    <v-card>
      <v-card-title>{{ t('settings.about') }}</v-card-title>
      <v-card-text>
        <v-list density="compact">
          <v-list-item :title="t('settings.version')">
            <template #append>
              <span class="text-medium-emphasis">v{{ packageJson.version }}</span>
            </template>
          </v-list-item>
          <v-list-item :title="t('settings.description')">
            <template #append>
              <span class="text-medium-emphasis text-right">{{ t('settings.descriptionText') }}</span>
            </template>
          </v-list-item>
          <v-list-item :title="t('settings.repo')">
            <template #append>
              <span class="text-medium-emphasis">github.com/BlazeSnow/Todo4Agent</span>
            </template>
          </v-list-item>
        </v-list>
      </v-card-text>
    </v-card>
  </div>
</template>

<style scoped>
/* 数据库路径可能较长，允许在任意字符处换行 */
.db-path {
  min-width: 0;
  word-break: break-all;
}
</style>