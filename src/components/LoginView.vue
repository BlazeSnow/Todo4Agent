<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { authLogin, authRegister, authStatus, setToken } from '../api'
import { onEnterSubmit } from '../ime'
import LocaleSwitch from './LocaleSwitch.vue'

const emit = defineEmits<{
  (e: 'logged-in', username: string): void
  (e: 'error', msg: string): void
}>()

const { t } = useI18n()

const username = ref('')
const password = ref('')
const busy = ref(false)
const errorMsg = ref('')
/** 存在仍在使用默认密码的账户时提示初始凭据 */
const showDefaultHint = ref(false)
/** 服务端是否允许注册（关闭时隐藏注册按钮） */
const allowRegister = ref(true)

onMounted(async () => {
  try {
    const s = await authStatus()
    showDefaultHint.value = s.default_password === true
    allowRegister.value = s.allow_register
  } catch {
    showDefaultHint.value = false
  }
})

async function doLogin() {
  if (!username.value.trim() || password.value.length < 4) return
  busy.value = true
  errorMsg.value = ''
  try {
    const r = await authLogin(username.value.trim(), password.value)
    setToken(r.token)
    emit('logged-in', r.username)
  } catch (e) {
    errorMsg.value = (e as Error).message
  } finally {
    busy.value = false
  }
}

async function doRegister() {
  if (!username.value.trim() || password.value.length < 4) return
  busy.value = true
  errorMsg.value = ''
  try {
    const r = await authRegister(username.value.trim(), password.value)
    setToken(r.token)
    emit('logged-in', r.username)
  } catch (e) {
    errorMsg.value = (e as Error).message
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="login-wrap">
    <v-card class="login-card" variant="outlined">
      <v-card-title class="text-center pt-6 pb-2">
        <v-icon icon="mdi-checkbox-marked-circle-outline" class="mr-1" color="primary" />
        Todo4Agent
      </v-card-title>
      <v-card-subtitle class="text-center pb-4">
        {{ t('login.subtitle') }}
      </v-card-subtitle>
      <v-card-text>
        <v-alert
          v-if="showDefaultHint"
          type="warning"
          density="compact"
          class="mb-4"
          :title="t('login.defaultAccount')"
        >
          {{ t('login.defaultAccountHint') }}
        </v-alert>
        <v-text-field
          v-model="username"
          :label="t('login.username')"
          autocomplete="username"
          autofocus
          @keydown.enter="onEnterSubmit($event, doLogin)"
        />
        <v-text-field
          v-model="password"
          :label="t('login.password')"
          type="password"
          autocomplete="current-password"
          @keydown.enter="onEnterSubmit($event, doLogin)"
        />
        <v-alert v-if="errorMsg" type="error" density="compact" class="mt-2">
          {{ errorMsg }}
        </v-alert>
      </v-card-text>
      <v-card-actions class="px-6 pb-6">
        <LocaleSwitch />
        <v-spacer />
        <v-btn v-if="allowRegister" variant="tonal" :loading="busy" @click="doRegister">{{ t('login.register') }}</v-btn>
        <v-btn color="primary" :loading="busy" @click="doLogin">{{ t('login.signIn') }}</v-btn>
      </v-card-actions>
    </v-card>
  </div>
</template>

<style scoped>
.login-wrap {
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(160deg, rgba(0, 168, 98, 0.08), rgba(0, 0, 0, 0.04));
  padding: 16px;
}
.login-card {
  width: 380px;
  max-width: 100%;
}
</style>