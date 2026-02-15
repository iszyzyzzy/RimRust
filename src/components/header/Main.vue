<template>
    <div>
        <n-flex>
            <n-dropdown trigger="click" :options="options" @select="handleSelect">
                <n-button quaternary style="padding: 5px;">文件</n-button>
            </n-dropdown>
            <n-button quaternary style="padding: 5px;" @click="showUserOrderModal = true">排序</n-button>
            <n-button quaternary style="padding: 5px;" @click="showAppConfigModal = true">设置</n-button>
        </n-flex>
        <ConfigModal v-model:show="showAppConfigModal" />
        <UserOrder v-model:show="showUserOrderModal" />
    </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import { useMessage } from 'naive-ui'

import { loadModsFromXml, loadFromGameConfig, saveModsToXml, saveToGameConfig, getSortedMods } from '@api/tauriFunc'
import { useBaseListStore } from '@store/baseList'

import ConfigModal from './Config.vue'
import UserOrder from './UserOrder.vue'

const baseList = useBaseListStore()
const message = useMessage()

const showAppConfigModal = ref(false)
const showUserOrderModal = ref(false)

const options = ref([
    { label: '读取指定xml', key: 'load' },
    { label: '读取当前xml', key: 'load-current' },
    { label: '保存为...', key: 'save-as' },
    { label: '保存至当前xml', key: 'save' },
    { type: 'divider' },
])
const handleSelect = async (key: string) => {
    switch (key) {
        case 'load':
            loadModsFromXml().then(() => {
                    message.success('加载成功');
                }
            ).catch(() => {
                    message.error('加载失败');
                })
            break
        case 'load-current':
            loadFromGameConfig().then(() => {
                    message.success('加载成功');
                }).catch(() => {
                    message.error('加载失败');
                })
            break
        // TODO 先这样吧，后面再研究怎么传递list的问题
        case 'save':
            let enabledMods = await getSortedMods();
            saveToGameConfig(enabledMods.list).then(() => {
                message.success('保存成功, 源文件已备份于同目录下');
            }).catch(() => {
                message.error('保存失败');
            })
            break
        case 'save-as':
            let enabledMods2 = await getSortedMods();
            saveModsToXml(enabledMods2.list).then(() => {
                message.success('保存成功');
            }).catch(() => {
                message.error('保存失败');
            })
            break
    }
}
const emit = defineEmits(['openConfig'])
</script>