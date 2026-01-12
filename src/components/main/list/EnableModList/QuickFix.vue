<script setup lang="ts">
import { SortError } from '@/api/types';
import { getSteamDBDataByPackageId } from '@/api/tauriFunc';
import { ref, watch } from 'vue';
import { useBaseListStore } from '@utils/store';
import { setEnableMod } from '@/components/utils/func/modOperation';
import { useMessage } from 'naive-ui';

const baseList = useBaseListStore();
const message = useMessage();

const available = ref(false);
const checking = ref(false);
let fixMethod: (() => void) | null = null;
const quick_fix_desc = ref('');

const { err } = defineProps<{
    err: SortError | null
}>()

const check = () => {
    if (err) {
        switch (true) {
            case 'MissingDependency' in err:
                checking.value = true;
                checkMissingDependency();
                break;
            default:
                available.value = false;
                checking.value = false;
                fixMethod = null;
        }
    }
}
const checkMissingDependency = async () => {
    if (!err || !('MissingDependency' in err)) {
        checking.value = false;
        return;
    }
    let res = Object.values(baseList.mods).find(m => m.packageId === err.MissingDependency[1]);
    if (res) {
        available.value = true;
        checking.value = false;
        quick_fix_desc.value = `启用 ${res!.displayName}`;
        fixMethod = () => {
            message.success(`快速修复: 启用 ${res!.displayName}`);
            setEnableMod([res!.id], true, baseList);
        }
        return;
    } 
    let res2 = await getSteamDBDataByPackageId(err.MissingDependency[1]);
    if (res2 && res2.length === 1) {
        available.value = true;
        checking.value = false;
        quick_fix_desc.value = `依赖未订阅，但是通过steamDB找到，可以在steam中打开"${res2![0].steamName}"的创意工坊页面`;
        fixMethod = () => {
            message.success(`快速修复: 通过steam打开${res2![0].steamName}的创意工坊页面`);
            window.open(res2![0].url, '_blank');
        }
        return;
    }
    available.value = false;
    checking.value = false;
    fixMethod = null;
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
        <n-popover trigger="hover" v-else-if="available">
            <template #trigger>
                <n-button text type="primary" @click="applyQuickFix">快速修复</n-button>
            </template>
            <span>{{ quick_fix_desc }}</span>
        </n-popover>
    </div>
</template>

<style scoped>
.quick-fix {
    width: 100%;
}
</style>