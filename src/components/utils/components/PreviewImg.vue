<template>
    <n-flex justify="center" align="center"  :style="{height: height + 'px', width: width + 'px'}">
        <n-image
            v-if="imgSrc"
            :src="imgSrc" 
            object-fit="contain" 
            :height="height" 
            :width="width" 
            size="small" 
            ref="picRef" 
            :preview-disabled="props.previewDisabled">
        </n-image>
        <n-flex justify="center" align="center" v-else>
                    <n-empty size="huge" description="看起来作者懒得放preview图" />
                </n-flex>
    </n-flex>
</template>

<script setup lang="ts">
import { convertFileSrc } from '@tauri-apps/api/core';
import { computedAsync } from '@vueuse/core'
import { findPreviewImage } from '../../../api/tauriFunc';

const props = defineProps<{
    path: string | null | undefined
    previewDisabled?: boolean
    height?: number
    width?: number
}>()

const imgSrc = computedAsync(async () => {

    if (props.path) {
        let res = await findPreviewImage(props.path + '/About/')
        if (res) {
            return convertFileSrc(res)
        } else {
            return null
        }
    } else {
        return null
    }
})

</script>