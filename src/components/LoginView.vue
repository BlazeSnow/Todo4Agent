<script setup lang="ts">
import { ref } from 'vue'
import { authLogin, authRegister, setToken } from '../api'

const emit = defineEmits<{
  (e: 'logged-in', username: string): void
  (e: 'error', msg: string): void
}>()

const username = ref('')
const password = ref('')
const busy = ref(false)
const errorMsg = ref('')

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
        为 Agent 设计的 MCP 任务清单
      </v-card-subtitle>
      <v-card-text>
        <p class="text-body-2 text-medium-emphasis mb-4">
          尚未创建用户时，注册将创建首个用户并接管现有本地数据。
        </p>
        <v-text-field
          v-model="username"
          label="用户名"
          autofocus
          @keydown.enter="doLogin"
        />
        <v-text-field
          v-model="password"
          label="密码（至少 4 位）"
          type="password"
          @keydown.enter="doLogin"
        />
        <v-alert v-if="errorMsg" type="error" density="compact" class="mt-2">
          {{ errorMsg }}
        </v-alert>
      </v-card-text>
      <v-card-actions class="px-6 pb-6">
        <v-spacer />
        <v-btn variant="tonal" :loading="busy" @click="doRegister">注册</v-btn>
        <v-btn color="primary" :loading="busy" @click="doLogin">登录</v-btn>
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