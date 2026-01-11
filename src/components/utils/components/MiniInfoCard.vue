<template>
    <n-card ref="rootCard" content-style="padding: 5px;" :bordered="showBorder">
        <n-collapse arrow-placement="right" v-if="showDetail">
            <n-collapse-item name="1">
                <n-scrollbar :style="detailHeight">
                <n-descriptions :column="2">
                    <n-descriptions-item label="packageId">
                        {{ data?.packageId }}
                    </n-descriptions-item>
                    <n-descriptions-item label="支持版本">
                        <n-flex>
                            <n-tag size="small" round v-for="version in supportedVersion" :key="version" :type="version ===
                                appConfigStore.game_version.split('.').splice(0, 2).join('.')
                                ? 'success'
                                : 'default'" :disabled="version !==
                                    appConfigStore.game_version.split('.').splice(0, 2).join('.')
                                    ">{{ version }}
                            </n-tag>
                        </n-flex>
                    </n-descriptions-item>
                </n-descriptions>
                <n-divider />
                <n-text style="white-space: pre-wrap">
                    {{ data?.description.get(appConfigStore.$state.game_version) }}
                </n-text>
                </n-scrollbar>
                <template #header>
                    <n-grid cols="6">
                        <n-gi :span="2">
                            <PreviewImg :path="data?.path" preview-disabled :height="previewImgHeight"
                                :width="170" />
                        </n-gi>

                        <n-gi :span="4">
                            <n-descriptions label-placement="top" :title="data?.displayName">
                                <n-descriptions-item label="原名">
                                    {{ data?.displayName }}
                                </n-descriptions-item>
                                <n-descriptions-item label="作者">
                                    {{ data?.author }}
                                </n-descriptions-item>
                            </n-descriptions>
                        </n-gi>
                    </n-grid>
                </template>
            </n-collapse-item>
        </n-collapse>
        <template v-else>
            <n-grid cols="6">
                <n-gi :span="2">
                    <PreviewImg :path="data?.path" preview-disabled :height="previewImgHeight" :width="170" />
                </n-gi>

                <n-gi :span="4">
                    <n-descriptions label-placement="top" :title="data?.displayName">
                        <n-descriptions-item label="原名">
                            {{ data?.displayName }}
                        </n-descriptions-item>
                        <n-descriptions-item label="作者">
                            {{ data?.author }}
                        </n-descriptions-item>
                    </n-descriptions>
                </n-gi>
            </n-grid>
        </template>
    </n-card>
</template>

<script setup lang="ts">
import { ComponentPublicInstance, computed, ref } from 'vue';
import { ModInner } from '../../../api/types';
import { useAppConfigStore } from '../store';
import PreviewImg from './PreviewImg.vue';
const { data, showDetail, showBorder, maxHeight } = defineProps<{
    data: ModInner | null | undefined
    showDetail?: boolean
    showBorder?: boolean
    maxHeight?: number
}>()

const detailHeight = computed(() => {
    if (!maxHeight) return {}
    return { maxHeight: `${maxHeight - 100}px` }
})

const appConfigStore = useAppConfigStore()
const rootCard = ref<ComponentPublicInstance>()

// PreviewImg :style="{height: previewImgHeight}"/>
/* const previewImgHeight = computed(() => {
    //console.log(rootCard.value?.$el.offsetHeight)
    if (!rootCard.value) return '80px'
    return `${rootCard.value.$el.offsetHeight}px`
}) */
const previewImgHeight = computed(() => {
    if (!rootCard.value) return 80
    return Number(rootCard.value.$el.offsetHeight)
})

const supportedVersion = computed(() => {
    //console.time('version-sort');
    const result = data?.supportedVersion.sort((a, b) => {
        // return b.localeCompare(a)
        return Number(b) - Number(a);
    });
    //console.timeEnd('version-sort');
    return result;
});

</script>