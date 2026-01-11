<script setup lang="ts">
import { computed, ref } from "vue";
import { useBaseListStore } from "../utils/store";
import { Id, ModInner } from "../../api/types";

import {
    HistoryOutlined,
    ChevronLeftRound,
    ChevronRightRound,
} from "@vicons/material";

const baseListStore = useBaseListStore();

interface DropdownOption {
    label?: string;
    key?: number;
    modId?: Id;
    type: "history" | "divider";
}

const selectedMod = defineModel<ModInner | null>('selectedMod', { required: false });
const history = ref<Id[]>([]);
const currentIndex = ref(-1);

const historyOptions = computed(() => {
    let options: DropdownOption[] = [];

    history.value.forEach((id, index) => {
        if (index === currentIndex.value) {
            options.push({
                type: "divider",
            });
            options.push({
                label: baseListStore.getModById(id)?.displayName,
                key: index,
                modId: id,
                type: "history",
            });
            options.push({
                type: "divider",
            });
            return;
        }
        options.push({
            label: baseListStore.getModById(id)?.displayName,
            key: index,
            modId: id,
            type: "history",
        });
    });

    return options.reverse();
});

const pushHistory = (id: Id) => {
    if (currentIndex.value < history.value.length - 1) {
        history.value = history.value.slice(0, currentIndex.value + 1);
    }
    if (history.value[currentIndex.value] !== id) {
        history.value.push(id);
        currentIndex.value++;
    }
};

export interface HistoryInst {
    pushHistory: (id: Id) => void;
}

defineExpose<HistoryInst>({
    pushHistory,
});

const handleHistoryLeft = () => {
    if (currentIndex.value > 0) {
        currentIndex.value--;
        const id = history.value[currentIndex.value];
        selectedMod.value = baseListStore.getModById(id);
    }
};
const historyLeftData = computed(() => {
    if (currentIndex.value > 0) {
        const prevId = history.value[currentIndex.value - 1];
        return baseListStore.getModById(prevId);
    }
    return null;
});

const handleHistoryRight = () => {
    if (currentIndex.value < history.value.length - 1) {
        currentIndex.value++;
        const id = history.value[currentIndex.value];
        selectedMod.value = baseListStore.getModById(id);
    }
};

const historyRightData = computed(() => {
    if (currentIndex.value < history.value.length - 1) {
        const nextId = history.value[currentIndex.value + 1];
        return baseListStore.getModById(nextId);
    }
    return null;
});

const handleHistorySelect = (key: number) => {
    currentIndex.value = key;
    const id = history.value[key];
    selectedMod.value = baseListStore.getModById(id);
};

</script>

<template>
    <n-flex align="center" style="height: 36px; padding: 0">
        <n-popover trigger="click" :show-arrow="false" style="max-height: 400px" scrollable>
            <template #trigger>
                <n-button size="small" quaternary :disabled="historyOptions.length === 0">
                    <template #icon>
                        <n-icon>
                            <HistoryOutlined />
                        </n-icon>
                    </template>
                </n-button>
            </template>
            <n-flex vertical>
                <div v-for="option in historyOptions" :key="option.key" style="width: 100%">
                    <n-divider v-if="option.type === 'divider'" style="margin: 4px 0" />
                    <n-popover v-else trigger="hover" placement="right" :width="600" content-style="padding: 0;">
                        <template #trigger>
                            <n-button quaternary :type="option.key === currentIndex ? 'primary' : 'default'"
                                style="padding: 4px 12px; justify-content: flex-start; width: 100%;"
                                @click="handleHistorySelect(option.key!)">
                                {{ option.label }}
                            </n-button>
                        </template>
                        <MiniInfoCard :data="baseListStore.getModById(option.modId!)" style="max-width: 600px" />
                    </n-popover>
                </div>
            </n-flex>
        </n-popover>
        <n-popover trigger="hover" :disabled="historyLeftData === null" content-style="padding: 0;">
            <template #trigger>
                <n-button size="small" quaternary :disabled="historyLeftData === null" @click="handleHistoryLeft">
                    <template #icon>
                        <n-icon>
                            <ChevronLeftRound />
                        </n-icon>
                    </template>
                </n-button>
            </template>
            <MiniInfoCard :data="historyLeftData"></MiniInfoCard>
        </n-popover>
        <n-popover trigger="hover" :disabled="historyRightData === null" content-style="padding: 0;">
            <template #trigger>
                <n-button size="small" quaternary :disabled="historyRightData == null" @click="handleHistoryRight">
                    <template #icon>
                        <n-icon>
                            <ChevronRightRound />
                        </n-icon>
                    </template>
                </n-button>
            </template>
            <MiniInfoCard :data="historyRightData"></MiniInfoCard>
        </n-popover>
    </n-flex>
</template>