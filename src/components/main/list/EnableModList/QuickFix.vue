<script setup lang="ts">
import { SortError, SortWarning } from '@/api/types';
import { getSteamDBDataByPackageId } from '@/api/tauriFunc';
import { ref, watch } from 'vue';
import { useBaseListStore, useScrollTo, useSelectMod } from '@utils/store';
import { setEnableMod } from '@/components/utils/func/modOperation';
import { useMessage } from 'naive-ui';

const baseList = useBaseListStore();
const scrollTo = useScrollTo();
const selectMod = useSelectMod();
const message = useMessage();

const available = ref(false);
const checking = ref(false);
let fixMethod: (() => void) | null = null;
const quick_fix_name = ref('');
const quick_fix_desc = ref('');

const { err } = defineProps<{
    err: SortError | SortWarning | null
}>()

const check = () => {
    if (err) {
        switch (true) {
            case 'MissingDependency' in err:
                checking.value = true;
                checkMissingDependency();
                break;
            case 'VersionMismatch' in err:
                checking.value = true;
                checkVersionMismatch();
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
        quick_fix_name.value = `启用 ${res!.displayName}`;
        quick_fix_desc.value = `依赖未启用，快速启用依赖模组 "${res!.displayName}"`;
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
        quick_fix_name.value = `在 steam 中打开`;
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
const checkVersionMismatch = () => {
    if (!err || !('VersionMismatch' in err)) {
        checking.value = false;
        return;
    }
    {
        available.value = true;
        checking.value = false;
        quick_fix_name.value = `跳转至`;
        quick_fix_desc.value = ``;
        fixMethod = () => {
            message.success(`快速修复: 跳转至${err.VersionMismatch[0]}`);
            scrollTo.scrollTo(err.VersionMismatch[0]);
            selectMod.selectMod(err.VersionMismatch[0]);
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
        <n-popover trigger="hover" v-else-if="available && quick_fix_desc !== ''">
            <template #trigger>
                <n-button text type="primary" @click="applyQuickFix">{{ quick_fix_name }}</n-button>
            </template>
            <span>{{ quick_fix_desc }}</span>
        </n-popover>
        <n-button v-else-if="available && quick_fix_desc === ''" text type="primary" @click="applyQuickFix">{{ quick_fix_name }}</n-button>
    </div>
</template>

<style scoped>
.quick-fix {
    width: 100%;
}
</style>