<template>
    <n-drawer v-model:show="show" :width="600" placement="right">
        <n-drawer-content title="---" :native-scrollbar="false">  
            <MiniInfoCard :data="sourceData" showDetail show-border/>
            <n-divider>
                <n-icon>
                    <ArrowDownOutlined />
                </n-icon>
            </n-divider>
            <MiniInfoCard :data="targetData" showDetail show-border/>
            <n-divider />
            <n-card v-if="customCalcResult">
                <n-grid x-gap="6" :cols="3">
                    <n-gi>
                        <n-statistic label="总分" tabular-nums>
                            <n-number-animation
                                ref="numberAnimationInstRef"
                                :from="0.0"
                                :to="customCalcResult.overall_score"
                                :precision="2"
                                :duration="1200"
                                />
                            <template #suffix>
                            / {{customCalcResult.theshold}}
                            </template>
                        </n-statistic>
                    </n-gi>
                    <n-gi :span="2">
                        <n-list>
                            <n-list-item>
                                
                            </n-list-item>
                        </n-list>
                    </n-gi>
                </n-grid>
            </n-card>
            <n-card v-else>
                <n-skeleton text :repeat="2" /> <n-skeleton text style="width: 60%" />
            </n-card>
            <n-divider />
            <n-flex>
                <n-button v-if="!('Matched' in status)" @click="handleConfirm(sourceData.id, targetData.id)">确认</n-button>
                <n-button @click="handleIgnore(sourceData.id)" >忽略</n-button>
                <n-button v-if="'Matched' in status" @click="handleRemove(sourceData.id)" >删除</n-button>
            </n-flex>
        </n-drawer-content>
    </n-drawer>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useBaseListStore } from '../../utils/store';
import { CustomCalcResult, Id } from '../../../api/types';
import { ArrowDownOutlined } from '@vicons/antd'
import MiniInfoCard from '../../utils/components/MiniInfoCard.vue';
import { confirmTranslation, tranUserIgnoreAdd, removeTranslation, translationMatchCustomCalc } from '../../../api/tauriFunc';
import { NumberAnimationInst, useMessage } from 'naive-ui';

const show = defineModel<boolean>('show')
const props = defineProps<{
    id: Id,
    targetId: Id
}>()
const message = useMessage()
const baseListStore = useBaseListStore()

const sourceData = computed(() => {
    return baseListStore.getModById(props.id)!
})

const targetData = computed(() => {
    return baseListStore.getModById(props.targetId)!
})

const status = computed(() => {
    return baseListStore.translationModData[sourceData.value.id]
})

const handleConfirm = (id: string, targetId: string) => {
    confirmTranslation(id, targetId).then(() => {
        message.success('确认成功')
        show.value = false
    })
    baseListStore.translationModData[id] = { type: 'Matched', value: targetId}
    //console.log('confirm', id, targetId)
}

const handleIgnore = (id: string) => {
    tranUserIgnoreAdd(id).then(() => {
        message.success('忽略成功')
        show.value = false
    })
    baseListStore.translationModData[id] = { type: 'Ignored', value: null}
    //console.log('ignore', id, targetId)
}

const handleRemove = (id: string) => {
    removeTranslation(id).then(() => {
        message.success('删除成功')
        show.value = false
    })
    delete baseListStore.$state.translationModData[id]
    //console.log('remove', id)
}

const customCalcResult = ref<CustomCalcResult | null>(null)
const numberAnimationInstRef = ref<NumberAnimationInst | null>(null)

watch(props,async (newVal) => {
    customCalcResult.value = null
    customCalcResult.value = await translationMatchCustomCalc(newVal.id, newVal.targetId)
})
</script>