<script setup lang="ts">
import { Id, ModInner } from '../../../api/types';
import ItemMod from './ItemMod.vue'
import ItemGroup from './ItemGroup.vue'
import { ComponentPublicInstance, computed, onMounted, onUnmounted, ref, watch, h, nextTick } from 'vue';
import { useMessage } from 'naive-ui'
import { changeModDisplayOrder, refreshModsData, reorderModsByName, resetModOrder, searchMods } from '../../../api/tauriFunc';
import VirtualList from '../../utils/components/VirtualList.vue';
import type { DragCrossListPayload, DragEndPayload, VirtualListInst } from '../../utils/components/VirtualListInterface';
import { useAppConfigStore, useBaseListStore, useScrollTo } from '../../utils/store';
import { KeyboardArrowDownRound } from '@vicons/material';
import buttonCheckbox from '@/components/utils/components/ButtonCheckbox.vue';

type Item = ModInner
const isMod = (item: Item): item is ModInner => {
    return 'displayName' in item;
}

const baseList = useBaseListStore()
const appConfig = useAppConfigStore()
const scrollTo = useScrollTo()
const props = defineProps<{
    selected?: Id,
    title?: string,
    active?: boolean,
    listId: number
}>()
const emit = defineEmits<{
    select: [id: Id],
    focus: [],
    newHighlight: [pattern: string[]],
    dragEnd: [payload: DragEndPayload],
    dragCrossList: [payload: DragCrossListPayload]
}>()
// TODO 1 搜索时非匹配项虚化而不是隐藏的功能
const searchValue = ref('')
const searchLoading = ref(false)
const filterList = ref<Id[]>([])
const filterEnabled = ref(false)

const isItemSelected = ref(false)

const handleClick = (id: Id) => {
    isItemSelected.value = true
    emit('select', id)
}

import { setEnableMod } from '../../utils/func/modOperation'

const handleDoubleClick = (id: Id) => {
    setEnableMod([id], !(baseList.getModById(id)?.enabled ?? false), baseList)
}

let showData = ref<Item[]>([])

const updateShowData = () => {
    let t: Item[] = []
        if (filterEnabled.value) {
            t = filterList.value.map((id) => baseList.getModById(id)!)
        } else {
            t = baseList.getModOrderd
        }
    console.log("showData update",t)
    showData.value = t
}

watch([filterEnabled, filterList], () => {
    updateShowData()
}, { immediate: true })

baseList.$subscribe(() => {
    updateShowData()
}, { deep: true })

const handleCardClick = (e: MouseEvent) => {
    // 如果点击的是卡片而不是项目
    if (e.target === rootRef.value?.$el) {
        isItemSelected.value = false
    }
    emit('focus')
}
const rootRef = ref<ComponentPublicInstance>()
const titleRef = ref<HTMLElement>()
const searchRef = ref<ComponentPublicInstance>()
const actionRef = ref<ComponentPublicInstance>()

const listMaxHeight = ref({
    height: "150px"
});

/* const listMaxHeight = computed(() => {
    return { maxHeight: `calc(100vh - 124px - 36px - 36px - ${
        (titleRef.value?.$el?.offsetHeight || 0) +
        (searchRef.value?.$el?.offsetHeight || 0) +
        (actionRef.value?.$el?.offsetHeight || 0)
        }px)` }
}) */

watch(() => showData.value, () => {
    //console.log(titleRef.value?.offsetHeight, searchRef.value?.$el?.offsetHeight, actionRef.value?.$el?.offsetHeight)
    listMaxHeight.value = {
        height: `calc(100vh - 124px - 36px - 36px + 8px - ${(titleRef.value?.offsetHeight || 0) + 8 +
            (searchRef.value?.$el?.offsetHeight || 0) + 8 +
            (actionRef.value?.$el?.offsetHeight || 0)
            }px)`
    }
    //console.log('listMaxHeight', listMaxHeight.value)
})

const virtualListInst = ref<VirtualListInst | null>()

// watch(() => props.scrollTo, (scrollTo) => {
//     if (scrollTo && virtualListInst.value && props.active) {
//         virtualListInst.value.scrollToKey(scrollTo)
//     }
// })
scrollTo.$subscribe((_, state) => {
    if (state.target_id && (state.special_list === null || state.special_list === "all")) {
        nextTick(() => {
            virtualListInst.value!.scrollToKey(state.target_id!)
        })
    }
})

const message = useMessage()
const handleKeyUp = () => {
    //console.log('handleUp')
    if (props.active) {
        const index = showData.value.findIndex((item) => {
            return isMod(item) && item.id === props.selected
        })
        if (index > 0) {
            handleClick((showData.value[index - 1] as ModInner).id)
            virtualListInst.value!.scrollToIndex(index - 1)
        } else if (index === 0) {
            message.info('已经到顶啦')
        }
    }
}
const handleKeyDown = () => {
    //console.log('handleDown')
    if (props.active) {
        const index = showData.value.findIndex((item) => {
            return isMod(item) && item.id === props.selected
        })
        if (index < showData.value.length - 1) {
            handleClick((showData.value[index + 1] as ModInner).id)
            virtualListInst.value!.scrollToIndex(index + 1)
        } else if (index === showData.value.length - 1) {
            message.info('已经到底啦')
        }
    }
}

const handleEnter = () => {
    let org = baseList.getModById(props.selected || '')
    if (!org) return
    setEnableMod([props.selected || ''], !org.enabled, baseList)
}

const handleKeyEvents = (e: KeyboardEvent) => {
    if (isItemSelected.value) {
        if (e.key === 'ArrowUp') {
            e.preventDefault()
            handleKeyUp()
        } else if (e.key === 'ArrowDown') {
            e.preventDefault()
            handleKeyDown()
        } else if (e.key === 'Enter') {
            e.preventDefault()
            handleEnter()
        }
    }
}

watch(() => props.selected, (selected) => {
    if (selected && virtualListInst.value && props.active) {
        // TODO 
        //virtualListInst.value.scrollTo({ key: selected })
    }
})

const highlightField = ref<Record<Id, string[]>>({})
const highlightPattern = ref<string[] | null>()

const handleSearch = async () => {
    searchLoading.value = true
    if (searchValue.value === '') {
        filterList.value = []
        highlightPattern.value = []
        highlightField.value = {}
        emit('newHighlight', [])
        filterEnabled.value = false
        searchLoading.value = false
        return
    } else {
        filterEnabled.value = true
        let res = await searchMods(searchValue.value, selectValue.value)
        //console.log(res)
        filterList.value = res.mods.map((mod) => mod.id)
        highlightPattern.value = res.highlight
        emit('newHighlight', res.highlight)
        highlightField.value = res.mods.map((mod) => {
            return {
                [mod.id]: mod.matched_fields
            }
        }).reduce((acc, cur) => {
            return {
                ...acc,
                ...cur
            }
        }, {})
        searchLoading.value = false
        return
    }
}

const selectOption = ref([
    {
        label: '标题',
        value: 'name'
    }, {
        label: '显示名称',
        value: 'display_name'
    }, {
        label: '描述',
        value: 'description'
    }, {
        label: '作者',
        value: 'author'
    }, {
        label: 'packageId',
        value: 'packageId'
    }
])

const selectValue = ref([
    'name',
    'display_name',
    'description',
    'author',
    'packageId'
])

const title = computed(() => {
    if (filterEnabled.value) {
        return `${props.title} [${showData.value.length}/${Object.keys(baseList.mods).length}]`
    }
    return `${props.title}[${showData.value.length}]`
})

const itemMaxWidth = ref(200);
const updateItemMaxWidth = () => {
    if (!searchRef.value?.$el) return
    itemMaxWidth.value = searchRef.value.$el.offsetWidth
    //console.log('itemMaxWidth', itemMaxWidth.value)
}

onMounted(() => {
    window.addEventListener('keydown', handleKeyEvents);
    const resizeObserver = new ResizeObserver(() => {
        updateItemMaxWidth();
    });

    resizeObserver.observe(rootRef.value?.$el);

    onUnmounted(() => {
        resizeObserver.disconnect();
    });
});

onUnmounted(() => {
    window.removeEventListener('keydown', handleKeyEvents);
});

const isCompatible = (t: string) => {
    return t === appConfig.game_version.split('.').splice(0, 2).join('.') || t === '*';
}

const titleOptions = ref([
    {
        type: 'double-check-button',
        label: '按字母顺序重排',
        key: 'resetByName',
        doubleCheckText: '重排会丢失原本自定义排序!',
        handle: () => {
            reorderModsByName().then((res) => {
                baseList.$patch((state) => {
                    state.modsOrder = res;
                })
                message.success('重排成功');
            }).catch(() => {
                message.error('重排失败');
            })
        }
    },
    {
        type: 'double-check-button',
        label: '强制刷新',
        key: 'forceRefresh',
        doubleCheckText: 'debug',
        handle: () => {
            refreshModsData().then(() => {
                message.success('刷新成功');
            }).catch(() => {
                message.error('刷新失败');
            })
        }
    },
    {
        type: 'checkbox',
        label: '仅显示当前游戏版本兼容的模组',
        key: 'filterCompatible',
        state: false,
        handle: (value: boolean) => {
            if (value) {
                // filterList.value = Object.values(baseList.mods).filter((mod) => {
                //     return mod.supportedVersion.some((version) => isCompatible(version))
                // }).map((mod) => mod.id)
                filterList.value = baseList.getModOrderd
                    .filter((mod) => mod.supportedVersion.some((version) => isCompatible(version)))
                    .map((mod) => mod.id)
                filterEnabled.value = true
            } else {
                filterList.value = []
                filterEnabled.value = false
            }
        }
    }
])

</script>

<template>
    <n-card ref="rootRef" @click="handleCardClick" style="height: calc(100vh - 124px - 18px);">
        <n-flex vertical align="center">
            <div style="position: relative;width: 100%;" ref="titleRef">
                <span style="display: block; text-align: center;">{{ title }}</span>
                <n-popover trigger="click" placement="bottom">
                    <template #trigger>
                        <n-button quaternary circle style="position: absolute; right: 0; top: 50%; transform: translateY(-50%);" size="small">
                            <template #icon>
                                <n-icon><KeyboardArrowDownRound /></n-icon>
                            </template>
                        </n-button>
                    </template>
                    <n-flex vertical :size="0">
                        <div v-for="option in titleOptions" :key="option.key">
                            <n-popconfirm v-if="option.type === 'double-check-button'"  @positive-click="option.handle">
                                <template #trigger>
                                    <n-button quaternary>{{ option.label }}</n-button>
                                </template>
                                {{ option.doubleCheckText }}
                            </n-popconfirm>
                            <n-button v-else-if="option.type === 'button'" quaternary @click="option.handle">{{ option.label }}</n-button>
                            <ButtonCheckbox v-else-if="option.type === 'checkbox'" :text="option.label" :handle-click="option.handle" v-model="option.state" />
                        </div>
                    </n-flex>
                </n-popover>
            </div>
            <n-input-group ref="searchRef">
                <n-select style="width: 20%;" :options="selectOption" multiple v-model:value="selectValue"
                    :render-tag="() => h('div')" :consistent-menu-width="false" @update:value="handleSearch"
                    size="small" />
                <n-input v-model:value="searchValue" placeholder="搜索" @update:value="handleSearch" size="small"
                    :loading="searchLoading" />
            </n-input-group>
            <VirtualList
                ref="virtualListInst"
                :style="listMaxHeight"
                :estimated-item-height="25"
                :items="showData"
                key-field="id"
                :items-style="'mod-list'"
                :data-list-id="props.listId"
                @drag-end="emit('dragEnd', $event)"
                @drag-cross-list="emit('dragCrossList', $event)"
            >
                <template #default="{ item, textColor }">
                    <ItemMod v-if="isMod(item)" :item="item" @click="handleClick" @double-click="handleDoubleClick"
                        :selected="item.id === props.selected" :highlight-pattern="highlightPattern" class="mod-item"
                        :id="item.id" :highlight-field="highlightField[item.id]" :max-width="itemMaxWidth" :text-color="textColor"/>
                    <ItemGroup v-else :item="item" />
                </template>
            </VirtualList>
        </n-flex>
    </n-card>
</template>

<style scoped>

</style>