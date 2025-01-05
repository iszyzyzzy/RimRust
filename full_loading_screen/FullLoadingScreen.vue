<template>
    <div class="loading-screen" v-if="isLoading">
        <n-card class="loading-card">
            <n-spin size="large">
                <template #description>
                    <n-text>{{ loadingMessage }}</n-text>
                </template>
            </n-spin>
        </n-card>
    </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { NCard, NSpin, NText } from 'naive-ui'

const isLoading = ref(false)
const loadingMessage = ref('')
let unlisten1: any = null
let unlisten2: any = null

onMounted(async () => {
    // Listen for start-loading event
    unlisten1 = await listen('start-loading', (event) => {
        isLoading.value = true
        loadingMessage.value = event.payload as string
    })

    // Listen for end-loading event
    unlisten2 = await listen('end-loading', () => {
        isLoading.value = false
        loadingMessage.value = ''
    })
})

onUnmounted(() => {
    // Clean up event listeners
    if (unlisten1) unlisten1()
    if (unlisten2) unlisten2()
})
</script>

<style scoped>
.loading-screen {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.6);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 9999;
}

.loading-card {
    padding: 24px;
    background-color: rgba(255, 255, 255, 0.9);
}
</style>