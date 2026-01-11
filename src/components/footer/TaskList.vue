<script setup lang="ts">
import { computed, watch } from 'vue';
import { BackgroudTask } from './Footer.vue';
import { DeleteOutlined } from '@vicons/antd';

const show = defineModel<boolean>('show')
const data = defineModel<Map<string,BackgroudTask>>('data', {
    default: () => new Map()
})

const clean = () => {
    /* data.value = data.value.filter((item) => item.status !== '已完成' && item.status !== '已结束') */
    for (const [key, value] of data.value) {
        if (value.status === '已完成' || value.status === '已结束') {
            data.value.delete(key)
        }
    }
}

const list = computed(() => {
    return Array.from(data.value.values())
})

watch(show, () => {
    console.log('show', data.value)
})
</script>

<template>
    <n-modal
        v-model:show="show"
    >
        <n-card
            style="width: 800px"
            :title="'任务列表' + (Object.values(data).length > 0 ? `(${Object.values(data).length})` : '')"
            :bordered="false"
            size="large"
            role="dialog"
            aria-modal="true"
        >
        <template #header-extra>
            <n-button circle @click="clean" size="small" text>
                <template #icon>
                    <n-icon><DeleteOutlined /></n-icon>
                </template>
            </n-button>
        </template>
            <n-virtual-list style="max-height: 400px" :item-size="25" :items="list" v-if="data.size > 0">
                <template #default="{ item }">
                        <n-flex :wrap="false" style="padding: 5px;" align="center">
                            <n-ellipsis style="width: 250px;">{{ item.name }}</n-ellipsis>
                            <n-ellipsis style="max-width: 300px; min-width: 60px;" v-if="!(item.status === '已完成' || item.status === '已结束' || item.status === '等待中')">{{ item.info }}</n-ellipsis>
                            <n-progress type="line" :percentage="item.progress" />
                            <n-ellipsis style="max-width: 300px; min-width: 60px;">{{ item.status }}</n-ellipsis>
                        </n-flex>
                </template>
            </n-virtual-list>
            <n-empty v-else description="真的很闲"/>
        </n-card>
    </n-modal>
</template>