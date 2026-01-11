<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useAppConfigStore } from '../utils/store';
import { listen } from '@tauri-apps/api/event';
import TaskListModal from './TaskList.vue';

export type BackgroudTask = {
    id: string;
    name: string;
    status: string;
    info: string;
    progress: number;
}

const backgroundTasks = ref<Map<string,BackgroudTask>>(new Map())
const latestUpdateTask = ref<BackgroudTask | null>(null)
let latestUpdateTaskTimeout: number | null = null
const appConfigStore = useAppConfigStore()

// 添加状态计数缓存
const taskStatusCount = ref({
    finished: 0,
    total: 0
})

// 优化后的 globalCount
const globalCount = computed<[number, number, number]>(() => {
    const { finished, total } = taskStatusCount.value
    const percentage = Number((total === 0 ? 0 : finished / total * 100).toFixed(2))
    return [finished, total, percentage]
})

const isMassiveUpdate = ref(false)
let massiveUpdateTimer: number | null = null

type TaskStatusUpdate = {
    New: BackgroudTask
} | {
    Status: [string, string]
} | {
    Info: [string, string]
} | {
    Progress: [string, number]
}

onMounted(() => {
    listen<TaskStatusUpdate[]>('task_status_update_many', (event) => {
        console.log('received task status update many', event)

        const updates = event.payload
        let lastUpdatedTask: BackgroudTask | null = null;

        const updatedTasks = new Map(backgroundTasks.value)

        let count = 0
        for (const task of updates) {
            if ('Status' in task) {
                const [id, status] = task.Status
                let old_task = updatedTasks.get(id) 
                if (old_task && old_task.status !== status) {
                    // 更新计数
                    if (isFinishedStatus(old_task.status) && !isFinishedStatus(status)) {
                        taskStatusCount.value.finished--
                    } else if (!isFinishedStatus(old_task.status) && isFinishedStatus(status)) {
                        taskStatusCount.value.finished++
                    }
                    old_task.status = status
                    lastUpdatedTask = old_task
                }
            } else if ('Info' in task) {
                const [id, info] = task.Info
                let old_task = updatedTasks.get(id)
                if (old_task && old_task.info !== info) {
                    old_task.info = info
                    lastUpdatedTask = old_task
                }
            } else if ('Progress' in task) {
                const [id, progress] = task.Progress
                let old_task = updatedTasks.get(id)
                if (old_task && old_task.progress !== progress) {
                    old_task.progress = Number(progress.toFixed(2))
                    lastUpdatedTask = old_task
                }
            } else if ('New' in task) {
                let payload = task.New
                payload.progress = Number(payload.progress.toFixed(2))
                updatedTasks.set(payload.id, payload)
                latestUpdateTask.value = payload
        
                taskStatusCount.value.total++
            }
            count++
        }
        backgroundTasks.value = updatedTasks

        if (lastUpdatedTask) {
            latestUpdateTask.value = lastUpdatedTask
        }

        if (updates.length > 25) {
            isMassiveUpdate.value = true
        }

        if (massiveUpdateTimer) clearTimeout(massiveUpdateTimer)
        massiveUpdateTimer = setTimeout(() => {
            isMassiveUpdate.value = false
        }, 200)
    });
})

// 添加状态判断辅助函数
function isFinishedStatus(status: string): boolean {
    return status === '已完成' || status === '已结束' || status === '休眠中' || status === '出错'
}

const isFinish = computed(() => {
    let t = !Array.from(backgroundTasks.value.values()).some(task => !isFinishedStatus(task.status))
    if (t) {
        if (latestUpdateTaskTimeout) clearTimeout(latestUpdateTaskTimeout)
        latestUpdateTaskTimeout = setTimeout(() => {
            latestUpdateTask.value = null
        }, 5000)
    }
    return t
})


onUnmounted(() => {
    taskStatusCount.value = { finished: 0, total: 0 }
    if (massiveUpdateTimer) clearTimeout(massiveUpdateTimer)
})

const showModal = ref(false)
</script>

<template>
    <n-flex style="padding: 5px;"justify="start" align="center" :wrap="false">
        <div style="white-space:nowrap;font-size: 14px;padding-left: 2px;">游戏版本 {{ appConfigStore.game_version }}</div>
        <n-divider vertical />
        <n-button text size="small" @click="showModal = !showModal">后台任务</n-button>
        <n-spin :size="14" v-if="!isFinish" />
        <transition name="fade" mode="out-in">
            <n-flex v-if="latestUpdateTask && !isMassiveUpdate" :inline="true" :wrap="false" align="center" style="font-size: 14px; max-width: 25vw;" :key="latestUpdateTask.id">
                <n-ellipsis style="white-space:nowrap; flex-shrink: 0; min-width: 0;max-width: 200px;">{{ latestUpdateTask.name }}</n-ellipsis>
                <n-progress type="line" style="flex-shrink: 0; max-width: 240px;" :percentage="latestUpdateTask.progress" />
                <n-ellipsis style="white-space:nowrap;flex-shrink: 0; min-width: 0; max-width: 150px;">{{ latestUpdateTask.info }}</n-ellipsis>
            </n-flex>
            <n-progress v-else-if="isMassiveUpdate" type="line" :percentage="globalCount[2]" indicator-placement="inside" style="flex-shrink: 0; max-width: 25vw;" key="massive-update">
                {{ globalCount[0] }} / {{ globalCount[1] }}
            </n-progress>
            <span v-else-if="globalCount[0] !== globalCount[1]" key="global-count"> {{ globalCount[0] }} / {{ globalCount[1] }} </span>
        </transition>
        <n-divider vertical />
    </n-flex>
    <TaskListModal v-model:show="showModal" v-bind:data="backgroundTasks" />
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
    transition: opacity 0.1s ease;
}

.fade-enter-from,
.fade-leave-to {
    opacity: 0;
}
</style>