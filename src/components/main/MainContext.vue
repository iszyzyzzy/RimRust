<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { openUrl } from '@tauri-apps/plugin-opener';

import { useBaseListStore, useDragStore, useSelectMod } from "../utils/store";
import { Id, ModInner } from "../../api/types";
import InfoCard from "./InfoCard.vue";
import ModList from "./list/ModList.vue";
import EnableModList from "./list/EnableModList";
import TranslationOverview from "./translation/overview.vue";
import TranslationDrawer from "./translation/drawer.vue";
import HistoryBlock from "./History.vue";
import { HistoryInst } from "./History.vue";
import { handleDragCrossList, handleDragEnd } from "./dragFunc";
import { DragCrossListPayload, DragEndPayload } from "../utils/components/VirtualListInterface";

const baseListStore = useBaseListStore();
const dragStore = useDragStore();
const selectModStore = useSelectMod();
const selectedModId = ref<Id | null>(null);
const selectedMod = computed({
    get: () => selectedModId.value ? baseListStore.getModById(selectedModId.value) : null,
    set: (val) => { selectedModId.value = val?.id || null; }
});
const highlightPattern = ref<string[]>([]);

const historyRef = ref<HistoryInst | null>(null);

const handleSelect = (id: Id) => {
    selectedModId.value = id;
    historyRef.value?.pushHistory(id);
};

const handleHighlight = (pattern: string[]) => {
    highlightPattern.value = pattern;
};

const scrollTo = ref<string>("");

const showTranslationCheck = ref(false);

const activeList = ref<number>(0);

const handleListFocus = (index: number) => {
    activeList.value = index;
};

const tranDrawer = ref(false);
const tranDrawerId = ref<[id: Id, targetId: Id]>(["", ""]);

const handleTranView = (id: Id, targetId: Id) => {
    tranDrawerId.value = [id, targetId];
    tranDrawer.value = true;
};

const handleTranSelect = (id: Id, targetId: Id) => {
    tranDrawerId.value = [id, targetId];
    tranDrawer.value = true;
};

const onDragCrossList = (payload: DragCrossListPayload) => {
    handleDragCrossList(payload, dragStore, baseListStore);
};

const onDragEnd = (payload: DragEndPayload) => {
    handleDragEnd(payload, dragStore, baseListStore);
}

selectModStore.$subscribe((_, state) => {
    if (state.mod_id !== null) {
        handleSelect(state.mod_id);
    }
});

</script>

<template>
    <n-grid :cols="5" :x-gap="14" :y-gap="1" style="height: 100%">
        <n-gi :span="5">
            <HistoryBlock ref="historyRef" v-model:selected-mod="selectedMod" />
        </n-gi>
        <n-gi :span="2">
            <n-card style="height: 100%" v-if="selectedMod === null">
                <n-flex justify="center" align="center" style="height: 100%">
                    <n-empty size="huge">
                        <template #default>
                            <span>点点什么吧</span>
                        </template>
                    </n-empty>
                </n-flex>
            </n-card>
            <InfoCard v-else style="height: 100%" :data="selectedMod" 
            @select-translation="handleTranSelect" :highlight-pattern="highlightPattern" />
        </n-gi>
        <n-gi :span="1">
                <ModList @select="handleSelect" :selected="selectedMod?.id"
                    :scrollTo="scrollTo" :active="activeList === 0" @focus="handleListFocus(0)" title="mods"
                    @new-highlight="handleHighlight" :list-id="1" @drag-cross-list="onDragCrossList"
                    @drag-end="onDragEnd"/>
        </n-gi>
        <n-gi :span="1">
<!--             <ModList :active="activeList === 1"
                @focus="handleListFocus(1)" /> -->
        </n-gi>
        <n-gi :span="1">
                <EnableModList @select="handleSelect" :selected="selectedMod?.id"
                    :scrollTo="scrollTo" :active="activeList === 2" @focus="handleListFocus(2)"
                    title="已启用" :list-id="3" @drag-cross-list="onDragCrossList"
                    @drag-end="onDragEnd"/>
        </n-gi>
        <n-gi :span="2">
            <n-flex align="center" style="height: 36px; padding: 0">
                <n-button secondary size="small" @click="showTranslationCheck = true">汉化管理</n-button>
                <!-- placeholder -->
            </n-flex>
        </n-gi>
        <n-gi :span="3">
            <n-flex justify="end" align="center" style="height: 36px; padding: 0">
                <n-button size="small" secondary>按钮1</n-button>
                <n-button size="small" secondary @click="openUrl('steam://rungameid/294100')">启动游戏</n-button>
                <n-button size="small" secondary>按钮3</n-button>
            </n-flex>
        </n-gi>
    </n-grid>
    <TranslationOverview v-model:show="showTranslationCheck" @view="handleTranView" />
    <TranslationDrawer v-model:show="tranDrawer" :id="tranDrawerId[0]" :target-id="tranDrawerId[1]" />
</template>
