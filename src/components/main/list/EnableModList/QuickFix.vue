<script setup lang="ts">
import { SortError } from '@/api/types';
import { ref, watch } from 'vue';
import { useBaseListStore } from '@utils/store';
import { setEnableMod } from '@/components/utils/func/modOperation';
import { useMessage } from 'naive-ui';

const baseList = useBaseListStore();
const message = useMessage();

const available = ref(false);
const checking = ref(false);
let fixMethod: (() => void) | null = null;

const { err } = defineProps<{
    err: SortError | null
}>()

const check = () => {
    if (err) {
        if ('MissingDependency' in err) {// 先只写这一个吧
            checking.value = true;
            checkMissingDependency();
        }
    }
}
// TODO 先直接用find了，如果有性能问题再来优化吧
const checkMissingDependency = async () => {
    if (!err || !('MissingDependency' in err)) {
        checking.value = false;
        return;
    }
    let res = Object.values(baseList.mods).find(m => m.packageId === err.MissingDependency[1]);
    if (res) {
        available.value = true;
        checking.value = false;
        fixMethod = () => {
            message.success(`快速修复: 启用 ${res!.displayName}`);
            setEnableMod([res!.id], true, baseList);
        }
    } else {
        available.value = false;
        checking.value = false;
        fixMethod = null;
    }
}

const applyQuickFix = () => {
    if (fixMethod) {
        fixMethod();
    }
}

watch(() => err, () => {
    available.value = false;
    checking.value = false;
    check();
}, { immediate: true })
</script>

<template>
    <div class="quick-fix">
        <span v-if="checking" style="color: blightblue;">...</span>
        <n-button text type="primary" v-else-if="available" @click="applyQuickFix">快速修复</n-button>
    </div>
</template>

<style scoped>
.quick-fix {
    width: 100%;
}
</style>