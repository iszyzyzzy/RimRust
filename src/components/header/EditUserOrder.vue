<script setup lang="ts">
import { ComponentPublicInstance, ref, watch } from 'vue';
import { ArrowForwardOutlined } from '@vicons/material';
import { ModInner, ModOrder, PackageId } from '../../api/types';
import { useBaseListStore } from '../utils/store';
import MiniInfoCard from '../utils/components/MiniInfoCard.vue';

const baseList = useBaseListStore();

const show = ref(false)

const target = ref<PackageId | null>(null)
const targetData = ref<ModInner[]>([])
const targetDataLoading = ref(false)
let abortController: AbortController | null = null;

const cancelTargetSearch = () => {
    if (abortController) {
        abortController.abort()
        abortController = null
        targetDataLoading.value = false
    }
}

watch(target, async (newVal) => {
    cancelTargetSearch()
    if (newVal) {
        targetDataLoading.value = true
        abortController = new AbortController();
        try {
            //const res = await baseList.mods.findByPackageId(newVal, abortController.signal)
            // TODO 更新这里
            const res: ModInner[] = []
            targetData.value = res
            targetDataLoading.value = false
        } catch (e: any) {
            if (e.name === 'AbortError') {
                return
            } else {
                throw e
            }
        }
    }
})

const data = ref<ModOrder[]>([])
const dataLoading = ref<boolean[]>([])
const dataData = ref<ModInner[][]>([])
const dataAbortControllers = ref<AbortController[]>([])

watch(data, async (newVal, oldVal) => {
    for(let i = 0; i < newVal.length; i++) {
        const current = newVal[i]
        if (current !== oldVal[i]) {
            if('First' in current || 'Last' in current) {
                delete dataData.value[i]
                dataLoading.value[i] = false
                continue
            }
            dataLoading.value[i] = true
            if (dataAbortControllers.value[i]) {
                dataAbortControllers.value[i].abort()
            }
            dataAbortControllers.value[i] = new AbortController()
            try {
                //const packageId = 'Before' in current ? current.Before : current.After
                //const res = await baseList.mods.findByPackageId(packageId, dataAbortControllers.value[i].signal)
                // TODO 更新这里
                const res: ModInner[] = []
                dataData.value[i] = res
                dataLoading.value[i] = false
            } catch (e: any) {
                if (e.name === 'AbortError') {
                    return
                } else {
                    throw e
                }
            }
        }
    }
})

interface openConfig {
    target: PackageId
    preDefine: ModOrder[]
}

const start = (config: openConfig) => {
    target.value = config.target
    data.value = config.preDefine
    show.value = true
}

defineExpose({
    start
})

export interface EditUserOrderInterface extends ComponentPublicInstance {
    start: (config: openConfig) => void
}

const emit = defineEmits(['finish'])

const orderOptions = ref([
    {
        label: '最前',
        value: 'First'
    },
    {
        label: '最后',
        value: "Last"
    },
    {
        label: '早于',
        value: "Before"
    },
    {
        label: '晚于',
        value: "After"
    }
])
const handleOrderOptionsChange = (index: number, value: string) => {
    // TODO 这里
    // switch(value) {
    //     case 'First':
    //         data.value[index] = { First: null }
    //         break
    //     case 'Last':
    //         data.value[index] = { Last: null }
    //         break
    //     case 'Before':
    //         data.value[index] = { Before: "" }
    //         break
    //     case 'After':
    //         data.value[index] = { After: "" }
    //         break
    // }
}
</script>


<template>
    <n-modal v-model:show="show" preset="card" size="medium" title="编辑自定义排序">
        <n-flex justify="center" align="center" style="width: 100%;">
            <n-card style="width: 45%">
                <n-flex vertical>
                <n-input-group>
                    <n-input-group-label>PackageId</n-input-group-label>
                    <n-input v-model:value="target"/>
                </n-input-group>
                <n-text v-if="targetData.length > 1 && !targetDataLoading">多个匹配项</n-text>
                <template v-if="!targetDataLoading" >
                    <MiniInfoCard v-for="item in targetData" :key="item.id" :data="item" />
                </template>
                <n-card v-else>
                    <n-skeleton text :repeat="2" /> <n-skeleton text style="width: 60%" />
                </n-card>
            </n-flex>
            </n-card>
            <n-divider vertical>
                <n-icon>
                    <ArrowForwardOutlined />
                </n-icon>
            </n-divider>
            <n-flex vertical>
                <n-card style="width: 45%" v-for="(item, index) in data">
                    <template #header>
                        <n-input-group>
                            <n-select :options="orderOptions" @update:value="(val:string) => handleOrderOptionsChange(index, val)" />
                            <n-input-group-label v-if="'Before' in item || 'After' in item">PackageId</n-input-group-label>
                            <n-input v-if="'Before' in item" v-model:value="item.Before"/>
                            <n-input v-if="'After' in item" v-model:value="item.After"/>
                        </n-input-group>
                    </template>
                    <n-flex vertical>
                        <n-text>多个匹配项</n-text>
                        <template v-for="inner in dataData[index]" v-if="!dataLoading[index]">
                            <MiniInfoCard v-if="'Before' in item || 'After' in item" :data="inner"/>
                        </template>
                        <n-card v-else>
                            <n-skeleton text :repeat="2" /> <n-skeleton text style="width: 60%" />
                        </n-card>
                    </n-flex>
                </n-card>
            </n-flex>
        </n-flex>
        <template #footer>
        尾部
        </template>
    </n-modal>
</template>