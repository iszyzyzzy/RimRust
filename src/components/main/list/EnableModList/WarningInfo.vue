<script setup lang="ts">
import { Id, SortWarning } from '@/api/types';
import { useBaseListStore } from '@/components/utils/store';
import { computed } from 'vue';

import QuickFix from './QuickFix.vue';

const baseList = useBaseListStore();

const { err, sequence, displayQuickFix = false } = defineProps<{
    err: SortWarning
    sequence?: Number
    displayQuickFix: Boolean
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
        case 'ConflictingOrders' in err:
            t += `无法找到合适的加载顺序: ${getModName(err.ConflictingOrders[0])} 和 ${getModName(err.ConflictingOrders[1])}`;
            break;
        case 'DuplicatePackageId' in err:
            t += `存在重复的包ID: ${err.DuplicatePackageId}, 游戏可能会加载错误的模组`;
            break;
        case 'VersionMismatch' in err:
            t += `模组 ${getModName(err.VersionMismatch[0])} 未显式支持当前游戏版本`;
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
        <div class="quick-fix-container" v-if="displayQuickFix">
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