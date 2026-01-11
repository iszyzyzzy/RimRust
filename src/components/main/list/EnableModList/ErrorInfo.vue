<script setup lang="ts">
import { Id, SortError } from '@/api/types';
import { useBaseListStore } from '@/components/utils/store';
import { computed } from 'vue';

import QuickFix from './QuickFix.vue';

const baseList = useBaseListStore();

const { err, sequence } = defineProps<{
    err: SortError
    sequence: number | null
}>();

const getModName = (id: Id) => {
    return baseList.getModById(id)?.displayName || 'err:未知模组';
}

const text = computed(()=> {
    let t = '';
    if (sequence) {
        t += `${sequence}. `;
    }
    switch (true) {
        case 'MissingDependency' in err:
            t += `缺失依赖: "${getModName(err.MissingDependency[0])}" 需要 "${err.MissingDependency[2]}"`;
            break;
        case 'CircularDependency' in err:
            t += `循环依赖: ${err.CircularDependency.join(' -> \n')} -> ${err.CircularDependency[0]}`;
            break;
        case 'IncompatibleMods' in err:
            t += `不兼容: ${getModName(err.IncompatibleMods[0])} 与 ${getModName(err.IncompatibleMods[1])}`;
            break;
        default:
            t += '未知错误';
    }
    return t;
})
</script>

<template>
    <div class="container" v-if="err">
        <div class="error-text-container">{{ text }}</div>
        <div class="quick-fix-container">
            <QuickFix :err="err" />
        </div>
    </div>
</template>

<style scoped>
.container {
    display: flex;
    align-items: center;
    justify-content: space-between;
}
.error-text-container {
    flex-grow: 1;
    flex-shrink: 1;
    min-width: 50px;
    overflow: hidden;
}
.quick-fix-container {
    display: flex;
    flex-direction: row-reverse;
    /* 从右向左排列 */
    flex-wrap: nowrap;
    flex-shrink: 0;
    margin-left: 8px;
    max-width: 50%;
}
</style>