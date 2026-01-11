<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import type { ComponentPublicInstance } from "vue";
import { ModInner } from "../../api/types";
import { autoTranslate, setModDisplayName } from "../../api/tauriFunc";
import {
    useBaseListStore,
    useAppConfigStore,
} from "../utils/store";
import Highlight from "../utils/components/Highlight.vue";
import PreViewPic from "../utils/components/PreviewImg.vue";

const props = defineProps<{
    data: ModInner | null;
    highlightPattern?: string[] | null;
}>();

const isEditing = ref(false);
const editValue = ref("");

const startEdit = (value: string) => {
    editValue.value = value;
    isEditing.value = true;
};

const baseList = useBaseListStore();
const appConfig = useAppConfigStore();

const endEdit = () => {
    if (!isEditing.value) {
        return;
    }
    isEditing.value = false;
    if (editValue.value === props.data?.displayName) {
        return;
    }

    // baseList.$state.mods.updateItem(props.data?.id || "", [
    //     {
    //         displayName: editValue.value,
    //     },
    // ]);
    const change: [string, { displayName: string }[]] = [props.data?.id || "", [{ displayName: editValue.value }]];
    baseList.handleModChanges([change]);
    setModDisplayName(props.data?.id || "", editValue.value);
};

const description = computed(() => {
    return props.data?.description.getWithVersion(appConfig.$state.game_version);
});

const translate = computed(() => {
    return baseList.getTranslation(props.data?.id || "");
});

const translateDescription = computed(() => {
    return translate.value?.description.getWithVersion(appConfig.$state.game_version);
});

const unconfirmedtranslation = computed(() => {
    //console.log(tran.$state.unconfirmed)
    let t = baseList.$state.translationModData[props.data?.id || ""];
    if (!t) return null;
    if (t.type === 'UnconfirmedMatches') {
        return t.value;
    }
    return null;
});

const translatePack = computed(() => {
    let t = baseList.$state.translationModData[props.data?.id || ""];
    if (!t) return false;
    if (t.type === 'Translation') {
        return true;
    }
    return false;
});

const totalRef = ref<ComponentPublicInstance>();
const picRef = ref<InstanceType<typeof PreViewPic>>();
const titleRef = ref<ComponentPublicInstance>();
const descriptionsRef = ref<ComponentPublicInstance>();

//const totalRef = ref<HTMLElement>();
//const picRef = ref<HTMLElement>();
//const titleRef = ref<HTMLElement>();
//const descriptionsRef = ref<HTMLElement>();

const maxHeight = ref({
    maxHeight: "150px",
});

const calcMaxHeight = () => {
    //console.log("calcMaxHeight", picRef.value?.$el.offsetHeight, titleRef.value?.$el.offsetHeight, descriptionsRef.value?.$el.offsetHeight)

    maxHeight.value = {
        maxHeight: `calc(100vh - 124px - 36px - 36px - 12px - 1px - ${
            (picRef.value?.$el?.offsetHeight || 0) +
            ((titleRef.value?.$el?.offsetHeight || 0))
            }px)`,
    };

    //console.log(maxHeight.value);
}

import { useMessage, type TabsInst } from 'naive-ui'

let descriptionTabRef = ref<TabsInst | null>(null);

const autoTranslateText = ref("");
const autoTranslatePreText = ref("");
const autoTranslateLoading = ref(false);
const autoTranslateTimeout = ref<number | null>(null);

const tabsValue = ref("origin");

watch(
    () => props.data,
    () => {
        console.log(props.data?.id);
        autoTranslateText.value = "";
        autoTranslatePreText.value = "";
        requestAnimationFrame(() => {
            calcMaxHeight();
        });
        nextTick(() => {
            if (translateDescription.value) {
                tabsValue.value = "translation";
            } else {
                tabsValue.value = "origin";
            }
            descriptionTabRef.value?.syncBarPosition();
        });
    },
    { immediate: true }
);

/* const supportedVersion = computed(() => {
    //console.time('version-sort');
    const result = props.data?.supportedVersion.sort((a, b) => {
        // return b.localeCompare(a)
        return Number(b) - Number(a);
    });
    //console.timeEnd('version-sort');
    return result;
}); */

const supportedVersion = computed(() => {
    // 创建新数组避免直接修改原数组
    const versions = [...(props.data?.supportedVersion || [])];
    // 使用本地化比较，专门处理版本号
    return versions.sort((a, b) => {
        const [aMajor, aMinor] = a.split('.').map(Number);
        const [bMajor, bMinor] = b.split('.').map(Number);
        
        if (aMajor !== bMajor) {
            return bMajor - aMajor;
        }
        return bMinor - aMinor;
    });
});

const descriptionsMaxWidth = ref({
    maxWidth: `${(totalRef.value?.$el.offsetWidth - 48) / 2}px`,
});

const message = useMessage();

const handleDescriptionTabChange = (name: string) => {
    if (name === "auto" && !autoTranslateText.value) {
        autoTranslateLoading.value = true;
        autoTranslateTimeout.value = window.setTimeout(() => {
            message.info("deepl翻译用时可能较长，尤其是在文本很长需要分段翻译的情况下，请耐心等待");
        }, 200); // 200ms后没反应说明cache miss，可能需要等待
        autoTranslate(description.value![1], 'auto', appConfig.$state.prefer_language)
            .then((res) => {
                if (res.code === 200) {
                    autoTranslatePreText.value = `由deepl自${res.source}翻译为${res.target}`
                    autoTranslateText.value = res.data;
                } else {
                    autoTranslateText.value = `翻译失败：code: ${res.code}\nmessage: ${res.message}`;
                }
            })
            .finally(() => {
                autoTranslateLoading.value = false;
                if (autoTranslateTimeout.value) {
                    clearTimeout(autoTranslateTimeout.value);
                    autoTranslateTimeout.value = null;
                }
            });
    }
};

</script>

<template>
    <n-card ref="totalRef" content-style="padding-down: 0">
        <template #cover>
            <PreViewPic ref="picRef" :path="props.data?.path" :height="300"/>
        </template>
        <template #header>
            <template v-if="!isEditing">
                <n-text ref="titleRef" @dblclick="startEdit(props.data?.displayName || '')">
                    {{ props.data?.displayName }}
                </n-text>
            </template>
            <template v-else>
                <n-input ref="titleRef" v-model:value="editValue" @blur="endEdit" @keyup.enter="endEdit" autofocus />
            </template>
        </template>
        <n-scrollbar :style="maxHeight">
            <n-descriptions :column="2" ref="descriptionsRef">
                <n-descriptions-item label="原名" ><n-ellipsis :style="descriptionsMaxWidth">
                        <Highlight :text="props.data?.name" :patterns="props.highlightPattern" />
                    </n-ellipsis></n-descriptions-item>
                <n-descriptions-item label="作者" ><n-ellipsis :style="descriptionsMaxWidth">
                        <Highlight :text="props.data?.author" :patterns="props.highlightPattern" />
                    </n-ellipsis></n-descriptions-item>
                <n-descriptions-item label="packageId" ><n-ellipsis :style="descriptionsMaxWidth">
                        <Highlight :text="props.data?.packageId" :patterns="props.highlightPattern" />
                    </n-ellipsis></n-descriptions-item>
                <n-descriptions-item label="支持版本" >
                    <n-flex>
                        <n-tag size="small" round v-for="version in supportedVersion" :key="version" :type="version ===
                                appConfig.game_version.split('.').splice(0, 2).join('.')
                                ? 'success'
                                : 'default'" 
                                :disabled="version !==
                                appConfig.game_version.split('.').splice(0, 2).join('.')
                            ">{{ version }}
                        </n-tag>
                    </n-flex>
                </n-descriptions-item>
                <n-descriptions-item label="汉化">
                    <template v-if="translate">
                        <n-button size="small" text @click="$emit('select-translation',props.data?.id , translate?.id)">
                        {{ translate?.displayName }}
                        </n-button>
                    </template>
                    <template v-else-if="unconfirmedtranslation">
                        <n-button size="small" text @click="$emit('select-translation', props.data?.id, unconfirmedtranslation[0][0])">
                            发现可能的汉化包
                        </n-button>
                    </template>
                    <template v-else-if="translatePack">
                        这就是汉化
                    </template>
                    <template v-else-if="props.data?.supportLanguages.get(appConfig.game_version)?.includes('zh') || 
                                        props.data?.supportLanguages.get(appConfig.game_version)?.includes('zh-TW')">
                        自带中文
                    </template>
                    <template v-else>
                        未找到/写死的中文/没有中文
                    </template>
                </n-descriptions-item>
            </n-descriptions>
            <n-tabs type="line" animated @update:value="handleDescriptionTabChange" ref="descriptionTabRef" 
                v-model:value="tabsValue">
                <n-tab-pane name="translation" tab="汉化包简介" v-if="translateDescription">
                    <Highlight :text="translateDescription[1]" :patterns="props.highlightPattern" style="white-space: pre-wrap" />
                </n-tab-pane>
                <n-tab-pane name="origin" :tab="`简介(${description?.[0]})`">
                    <template v-if="description">
                        <!-- <n-text depth="3">{{ description[0] }}</n-text> -->
                        <!-- <n-scrollbar :style="descriptionMaxHeight" style="white-space: pre-wrap;">{{ description[1] }}</n-scrollbar> -->
                        <!-- <n-text style="white-space: pre-wrap;">{{ description[1] }}</n-text> -->
                        <Highlight :text="description[1]" :patterns="props.highlightPattern"
                            style="white-space: pre-wrap" />
                    </template>
                    <n-empty size="huge" v-else>
                        <template #default>
                            <n-text>完全没有简介也是神人了</n-text>
                        </template>
                    </n-empty>
                </n-tab-pane>
                <n-tab-pane name="auto" tab="机翻" v-if="description">
                    <template v-if="description">
                        <template v-if="autoTranslateLoading">
                            <n-skeleton text :repeat="2" /> <n-skeleton text style="width: 60%" />
                        </template>
                        <Highlight :text="autoTranslateText" :pre-text="autoTranslatePreText" :patterns="props.highlightPattern" style="white-space: pre-wrap" />
                    </template>
                    <n-empty size="huge" v-else>
                        <template #default>
                            <n-text>啥也没有我翻个蛋</n-text>
                        </template>
                    </n-empty>
                </n-tab-pane>
            </n-tabs>
        </n-scrollbar>
    </n-card>
</template>
