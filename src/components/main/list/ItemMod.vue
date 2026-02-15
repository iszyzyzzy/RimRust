<script setup lang="ts">
import { computed, h, onMounted, onUnmounted, ref, watch } from 'vue';
import { NFlex, useMessage, useModal } from 'naive-ui';
import {ErrorOutlineRound, WarningAmberRound } from '@vicons/material'

import { ModInner, Id, SortWarning, SortError } from '../../../api/types';
import cHighlight from '../../utils/components/Highlight.vue';
import ErrorInfo from './EnableModList/ErrorInfo.vue';
import WarningInfo from './EnableModList/WarningInfo.vue';

const props = defineProps<{
    item: ModInner
    selected?: boolean
    highlightPattern?: string[] | null
    highlightField?: string[] | null
    warning?: SortWarning[]
    error?: SortError[]
    maxWidth?: number
    textColor?: string | null
}>()

const maxWidth = computed(() => {
    return props.maxWidth || 200
})

const emit = defineEmits<{
    click: [id: Id]
    doubleClick: [id: Id]
}>()

const handleClick = () => {
    emit('click', props.item.id)
}

const handleDoubleClick = () => {
    emit('doubleClick', props.item.id)
}

/* const display_name = computed(() => {
    if (props.highlightPattern) {
        return props.item.displayName.replace(props.highlightPattern, '<span class="highlight">$&</span>')
    }
    return props.item.displayName
}) */

const x = ref(0)
const y = ref(0)
const showDropdown = ref(false)
const dropdownOptions = [
    { label: '打开文件夹', key: 'openInFolder' },
    { label: '在浏览器中打开', key: 'openInBrowser' },
    { label: '在steam中打开', key: 'openInSteam' }
]

const handleRightClick = (e: MouseEvent) => {
    x.value = e.clientX
    y.value = e.clientY
    showDropdown.value = true
}

const onClickoutside = () => {
    showDropdown.value = false
}

import { openPath, openUrl } from '@tauri-apps/plugin-opener';
import { getSteamDBDataByPackageId } from '../../../api/tauriFunc';
import MiniInfoCardForSteamdb from '../../utils/components/MiniInfoCardForSteamdb.vue';

const message = useMessage()
const modal = useModal()

const selectOneModal = ref(false)

const handleDropdownSelect = async (key: string) => {
    if (key === 'openInFolder') {
        await openPath(props.item.path);
    } else if (key === 'openInSteam' || key === 'openInBrowser') {
        let id: string | undefined
        // 三种可能
        // 1. 路径里有workshop\\content\\294100\\ 后面的数字是mod的id
        // 2. 路径里没有，尝试用packageId请求steamdb，如果有就打开，多个的话先按照author和name尝试匹配，再不行让用户选择
        // 3. 都没有，提示
        if (props.item.path.includes('workshop\\content\\294100\\')) {
            id = props.item.path.match(/workshop\\content\\294100\\(\d+)/)![1]
        } else {
            const data = await getSteamDBDataByPackageId(props.item.packageId)
            if (data.length === 1) {
                id = data[0].url?.split('?id=')[1]
            } else if (data.length > 1) {
                let find = false
                for (const item of data) {
                    let t_author = item.authors
                    if (typeof t_author === 'string') {
                        t_author = [t_author]
                    }
                    t_author = t_author?.join(", ")
                    if (t_author === props.item.author && item.name === props.item.name) {
                        id = item.url?.split('?id=')[1]
                        find = true
                        break
                    }
                }
                if (!find) {
                    selectOneModal.value = true

                    modal.create({
                        title: '找到多个可能的选项',
                        preset: 'card',
                        show: selectOneModal.value,
                        content: () => {
                            return h(
                                NFlex,
                                { vertical: true },
                                {
                                    default: () => data.map((item) => {
                                        return h(
                                            MiniInfoCardForSteamdb,
                                            {
                                                data: item,
                                                onClick: async () => {
                                                    id = item.url?.split('?id=')[1]
                                                    if (key === 'openInSteam') {
                                                        await openUrl(`steam://url/CommunityFilePage/${id}`);
                                                    } else if (key === 'openInBrowser') {
                                                        await openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${id}`);
                                                    }
                                                    selectOneModal.value = false
                                                }
                                            }
                                        )
                                    })
                                }
                            )
                        }
                    })

                    return
                }
            } else {
                message.warning("找不到steam workshop id")
                return
            }
        }

        if (key === 'openInSteam') {
            await openUrl(`steam://url/CommunityFilePage/${id}`);
        } else if (key === 'openInBrowser') {
            await openUrl(`https://steamcommunity.com/sharedfiles/filedetails/?id=${id}`);
        }
    }
    showDropdown.value = false
}

// 计算是否需要使用折叠标签效果
const useCollapsedTags = ref(false);
const tagContainerRef = ref<HTMLElement>();

import { getTextWidth } from '@utils/func/textWidth';

// 监听标签区域宽度变化
const checkTagWidth = () => {
    if (tagContainerRef.value) {
        const containerWidth = tagContainerRef.value.parentElement?.offsetWidth || 200;
        const tagWidth = props.highlightField?.reduce((total, tag) => {
            return total + getTextWidth(tag, { });
        }, 0) || 0;
        useCollapsedTags.value = tagWidth > containerWidth / 3;
    }
};

// 转换高亮字段为可显示的标签
const highlightTags = computed(() => {
    if (!props.highlightField || props.highlightField.length === 0) return [];
    return props.highlightField;
});

// 计算显示的标签数和隐藏的标签数
const displayedTags = computed(() => {
    if (!useCollapsedTags.value || highlightTags.value.length <= 1) {
        return highlightTags.value;
    }
    // 当需要折叠时，只显示第一个标签
    return highlightTags.value.slice(0, 1);
});

const hiddenTagsCount = computed(() => {
    if (!useCollapsedTags.value) return 0;
    return Math.max(0, highlightTags.value.length - 1);
});

// 窗口大小变化时重新检查
onMounted(() => {
    checkTagWidth();
    const resizeObserver = new ResizeObserver(checkTagWidth);

    resizeObserver.observe(tagContainerRef.value!.parentElement!);

    onUnmounted(() => {
        resizeObserver.disconnect();
    });
});

// 在标签列表变化时重新检查
watch(() => props.highlightField, () => {
    checkTagWidth();
}, { deep: true });

/* 
pub enum SearchField {
    Id,
    Name,
    DisplayName,
    Description,
    Author,
    PackageId,
}
 */

const tabMap = ref<Record<string, string>>({
    'id': 'ID',
    'name': '原名',
    'displayName': '显示名称',
    'description': '描述',
    'author': '作者',
    'packageId': 'packageId',
})

const textColorStyle = computed(() => {
    return props.textColor ? { color: props.textColor } : { color: 'var(--text-color-2)' };
});

const statusTag = computed(() => {
    if (props.error && props.error.length > 0) {
        return 'error'
    } else if (props.warning && props.warning.length > 0) {
        return 'warning'
    }
    return null
})

</script>

<template>
    <n-el 
        tag="div"
        :style="textColorStyle"
        class="item" 
        :class="{ selected: props.selected }" 
        @click="handleClick"
        @dblclick="handleDoubleClick"
        @click.right.prevent="handleRightClick">
        <div class="item-content" :style="{ width: `${maxWidth}px` }">
            <div class="name-container">
                <n-ellipsis :tooltip="false">
                    <cHighlight :text="props.item.displayName" :patterns="props.highlightPattern" />
                </n-ellipsis>
            </div>
            <div class="tags-container" ref="tagContainerRef">
                <!-- 当有隐藏标签时，显示+N项按钮 -->
                <n-popover v-if="hiddenTagsCount > 0" trigger="hover" placement="top">
                    <template #trigger>
                        <n-tag size="small" :bordered="false" type="info" class="more-tag">
                            +{{ hiddenTagsCount }}项
                        </n-tag>
                    </template>

                    <!-- Popover内容：显示所有标签 -->
                    <div class="popover-tags">
                        <n-flex>
                            <n-tag v-for="(tag, index) in highlightTags" :key="index" size="small" :bordered="false"
                            type="success">
                            {{ tabMap[tag] || tag }}
                        </n-tag>
                        </n-flex>
                    </div>
                </n-popover>

                <!-- 显示可见标签 -->
                <n-tag v-for="(tag, index) in displayedTags" :key="index" size="small" :bordered="false" type="success"
                    class="highlight-tag">
                    {{ tabMap[tag] || tag }}
                </n-tag>

                <!-- 受控tag -->
                <!-- <n-tag v-if="statusTag === 'error'" size="small" type="error">
                    <n-icon><ErrorOutlineRound /></n-icon>
                </n-tag>
                <n-tag v-else-if="statusTag === 'warning'" size="small" type="warning">
                    <n-icon><WarningAmberRound /></n-icon>
                </n-tag> -->
                <n-popover 
                    v-if="statusTag" trigger="hover" placement="top">
                    <template #trigger>
                        <n-tag size="small" :bordered="false" :type="statusTag" class="highlight-tag">
                            <n-icon>
                                <component :is="statusTag === 'error' ? ErrorOutlineRound : WarningAmberRound" />
                            </n-icon>
                        </n-tag>
                    </template>

                    <div class="popover-tags">
                        <div v-for="(err, index) in props.error" :key="index" style="display: block; margin-bottom: 8px;">
                            <ErrorInfo :err="err" display-quick-fix/>
                        </div>
                        <div v-for="(warn, index) in props.warning" :key="index" style="display: block; margin-bottom: 8px;">
                            <WarningInfo :err="warn" display-quick-fix/>
                        </div>
                    </div>
                </n-popover>
            </div>
        </div>
        <n-dropdown placement="bottom-start" trigger="manual" :x="x" :y="y" :options="dropdownOptions"
            :show="showDropdown" :on-clickoutside="onClickoutside" @select="handleDropdownSelect" />
    </n-el>
</template>

<style scoped>
.item {
    height: 25px;
    transition: background-color 0.1s ease;
    user-select: none;
}

.item:hover {
    background-color: #2D2D30;
}

body.dragging .item:hover {
    background-color: inherit;
}

.selected {
    background-color: #54697B;
}

.selected:hover {
    background-color: #54697B;
}

body.dragging .selected:hover {
    background-color: #54697B;
}

.item-content {
    display: flex;
    width: 100%;
    height: 100%;
    align-items: center;
    justify-content: space-between;
    overflow: hidden;
}

.name-container {
    flex-grow: 1;
    flex-shrink: 1;
    min-width: 50px;
    overflow: hidden;
}

.tags-container {
    display: flex;
    flex-direction: row-reverse;
    /* 从右向左排列 */
    flex-wrap: nowrap;
    flex-shrink: 0;
    margin-left: 8px;
    max-width: 50%;
}

.tags-scrolling {
    display: flex;
    flex-direction: row-reverse;
    white-space: nowrap;
}

.highlight-tag {
    margin-left: 4px;
    font-size: 0.8em;
}

.more-tag {
    margin-left: 4px;
    font-size: 0.8em;
    cursor: pointer;
}

.popover-tag {
    margin: 2px 0;
}
</style>
