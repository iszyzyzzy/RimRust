<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch, h, ComponentPublicInstance, nextTick } from 'vue';
import { useMessage, useModal, NInput, NRadioGroup, NRadio } from 'naive-ui';
import {isEqual} from 'lodash-es';
import {ErrorOutlineFilled, WarningAmberFilled, CheckOutlined} from '@vicons/material'

import { Id, ModInner, SortResult } from '@api/types';
import { DragCrossListPayload, DragEndPayload, VirtualListInst } from '@utils/components/VirtualListInterface';
import { useBaseListStore, useInitdStore } from '@utils/store';
import { getSortedMods, searchMods } from '@api/tauriFunc';
import { setEnableMod } from '@utils/func/modOperation';

import ItemMod from './../ItemMod.vue'
import VirtualList from '@utils/components/VirtualList.vue';
import EditUserOrder, { EditUserOrderInterface } from '@components/header/EditUserOrder.vue';
import ErrorInfo from './ErrorInfo.vue';

const props = defineProps<{
    selected?: Id,
    title?: string,
    scrollTo?: string,
    active?: boolean,
    listId?: number
}>()
const emit = defineEmits<{
    select: [id: Id],
    focus: [],
    newHighlight: [pattern: string[]],
    itemMoved: [fromListId: number, toListId: number, itemId: Id],
    dragEnd: [payload: DragEndPayload],
    dragCrossList: [payload: DragCrossListPayload]
}>()

// 基础数据
const baseListStore = useBaseListStore();
const searchValue = ref('');
const searchLoading = ref(false);
const isItemSelected = ref(false);

// 搜索相关
const filterList = ref<Id[]>([]);
const filterEnabled = ref(false);
const highlightField = ref<Record<Id, string[]>>({});
const highlightPattern = ref<string[] | null>();

// 排序相关
const sortedModsList = ref<ModInner[]>([]);
const sortResult = ref<SortResult>({ list: [], error: [], warning: [], info: [] });

// 显示数据（带搜索过滤）
const showData = computed(() => {
    let res
    if (filterEnabled.value) {
        // 使用搜索结果过滤，但只显示已启用的模组
        res = filterList.value
            .map(id => baseListStore.getModById(id))
            .filter(mod => mod && mod.enabled) as ModInner[];
    } else {
        // 显示所有已排序的已启用模组
        res = sortedModsList.value;
    }
    //console.log('Show data updated:', res);
    return res;
});

// 模态框控制
const showSortOrderModal = ref(false);
const draggingItemId = ref<Id>('');
const targetItemId = ref<Id>('');
const sortOrderType = ref<'before' | 'after'>('after');

// 加载状态
const loading = ref(false);

// 获取已排序的模组列表
const fetchSortedMods = async () => {
    loading.value = true;
    try {
        const result = await getSortedMods();
        sortResult.value = result;

        // 根据排序结果构建已排序模组列表
        const sortedIds = result.list;
        const newSortedList: ModInner[] = [];

        for (const id of sortedIds) {
            const mod = baseListStore.getModById(id);
            if (mod) {
                newSortedList.push(mod);
            } else {
                console.warn('id not found', id);
            }
        }

        sortedModsList.value = newSortedList;
        //console.log('Fetched sorted mods:', sortedModsList.value);

    } catch (error) {
        console.error('获取排序模组失败:', error);
        message.error('获取排序模组失败');
    } finally {
        loading.value = false;
    }
};


const initd = useInitdStore();

onMounted(async () => {
    await initd.wait_for_init();
    // message.info('第一次排序会比较慢，请耐心等待');
    fetchSortedMods();


});

onUnmounted(() => {

});

// 监听所有模组的启用状态变化
let cachedEnabledMods: Record<Id, boolean> = {};
baseListStore.$subscribe((_, state) => {
    const newEnabledMods = Object.entries(state.mods).filter(([id, inner]) => [id, inner.enabled]).reduce((acc, [id, inner]) => {
        acc[id] = inner.enabled;
        return acc;
    }, {} as Record<Id, boolean>);
    if (!isEqual(cachedEnabledMods, newEnabledMods)) {
        cachedEnabledMods = newEnabledMods;
        fetchSortedMods();
    }
});

// 处理项目点击
const handleClick = (id: Id) => {
    isItemSelected.value = true;
    emit('select', id);
};

// 拖拽和排序操作
const virtualListInst = ref<VirtualListInst | null>(null);

// UI控制
const rootRef = ref();
const titleRef = ref<HTMLElement>();
const searchRef = ref<ComponentPublicInstance>();
const actionRef = ref<ComponentPublicInstance>();

const listMaxHeight = ref({
    height: "150px"
});

watch(() => showData.value, () => {
    listMaxHeight.value = {
        height: `calc(100vh - 124px - 36px - 36px + 8px - 20px - ${(titleRef.value?.offsetHeight || 0) + 8 +
            (searchRef.value?.$el?.offsetHeight || 0) + 8 +
            (actionRef.value?.$el?.offsetHeight || 0)
            }px)`
    };
    nextTick(() => {
        //initSortable();
    });
});

// 处理卡片点击
const handleCardClick = (e: MouseEvent) => {
    if (e.target === rootRef.value?.$el) {
        isItemSelected.value = false;
    }
    emit('focus');
};

const handleEnter = async () => {
    let org = baseListStore.getModById(props.selected || '')
    if (!org) return
    setEnableMod([org.id], !org.enabled, baseListStore)
}

const handleDoubleClick = (id: Id) => {
    setEnableMod([id], !(baseListStore.getModById(id)?.enabled ?? false), baseListStore)
}

const handleKeyUp = () => {
    //console.log('handleUp')
    if (props.active) {
        const index = showData.value.findIndex((item) => {
            return isMod(item) && item.id === props.selected
        })
        if (index > 0) {
            handleClick((showData.value[index - 1] as ModInner).id)
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

        } else if (index === showData.value.length - 1) {
            message.info('已经到底啦')
        }
    }
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

onMounted(() => {
    window.addEventListener('keydown', handleKeyEvents);
});

onUnmounted(() => {
    window.removeEventListener('keydown', handleKeyEvents);
});

const message = useMessage();
const modal = useModal();

// 处理用户选择排序规则后的操作
const handleSortOrderConfirm = async () => {
    if (!draggingItemId.value || !targetItemId.value) return;

    // 关闭模态框
    showSortOrderModal.value = false;

    // 获取模组包ID
    const draggingMod = baseListStore.getModById(draggingItemId.value);
    const targetMod = baseListStore.getModById(targetItemId.value);

    if (!draggingMod || !targetMod) {
        message.error('无法获取模组信息');
        return;
    }

    try {
        // 更新排序规则
        const packageId = targetMod.packageId;
        // 这里应该调用后端API来更新用户自定义排序规则
        // API忘写了，先留个TODO
        // TODO: 调用更新用户自定义排序规则的API

        message.info(`设置 ${draggingMod.displayName} ${sortOrderType.value === 'after' ? '在' : '在'} ${targetMod.displayName} ${sortOrderType.value === 'after' ? '之后' : '之前'} 加载`);

        // 重新获取排序列表
        await fetchSortedMods();
    } catch (error) {
        console.error('设置排序规则失败:', error);
        message.error('设置排序规则失败');
    }
};

// 处理搜索功能
const handleSearch = async () => {
    searchLoading.value = true;
    try {
        if (searchValue.value === '') {
            // 清空搜索
            filterList.value = [];
            highlightPattern.value = [];
            highlightField.value = {}
            emit('newHighlight', []);
            filterEnabled.value = false;
        } else {
            // 启用搜索过滤
            filterEnabled.value = true;
            // 调用后端搜索API
            let res = await searchMods(searchValue.value, selectValue.value);
            // 获取搜索结果
            filterList.value = res.mods.map((mod) => mod.id);
            highlightPattern.value = res.highlight;
            emit('newHighlight', res.highlight);
            // 保存字段高亮信息
            highlightField.value = res.mods.map((mod) => {
                return {
                    [mod.id]: mod.matched_fields
                }
            }).reduce((acc, cur) => {
                return {
                    ...acc,
                    ...cur
                }
            }, {});
        }
    } catch (error) {
        console.error('搜索失败:', error);
        message.error('搜索失败');
    } finally {
        searchLoading.value = false;
    }
};


// 标题显示
const title = computed(() => {
    if (filterEnabled.value) {
        return `${props.title || '已启用'} [${showData.value.length}/${sortedModsList.value.length}]`;
    }
    return `${props.title || '已启用'} [${sortedModsList.value.length}]`;
});

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

const itemMaxWidth = computed(() => {
    return searchRef.value?.$el?.offsetWidth || 200;
}) 

type Item = ModInner
const isMod = (item: Item): item is ModInner => {
    return 'displayName' in item;
}
</script>

<template>
    <div>
        <n-card ref="rootRef" @click="handleCardClick" style="height: calc(100vh - 124px - 18px);">
            <n-flex vertical align="center">
                <span ref="titleRef">{{ title }}</span>
                <n-input-group ref="searchRef">
                    <n-select style="width: 20%;" :options="selectOption" multiple v-model:value="selectValue"
                        :render-tag="() => h('div')" :consistent-menu-width="false" @update:value="handleSearch"
                        size="small" />
                    <n-input v-model:value="searchValue" placeholder="搜索" @update:value="handleSearch" size="small"
                        :loading="searchLoading" />
                </n-input-group>
                <n-spin :show="loading" style="width: 100%;">
                    <div v-if="showData.length !== 0"></div>
                    <VirtualList
                        ref="virtualListInst"
                        v-if="showData.length !== 0" 
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
                    <n-empty v-else description="没有已启用的模组" />
                </n-spin>
                <div style="height: 28px;">
                    <n-popover
                        v-if="sortResult.error.length > 0"
                        placement="top"
                        trigger="hover"
                        style="max-width: 50vw; max-height: 35vh;"
                        scrollable
                    >
                        <template #trigger>
                            <n-tag type="error">
                                <template #icon>
                                    <n-icon><ErrorOutlineFilled /></n-icon>
                                </template>
                                {{ sortResult.error.length }}
                            </n-tag>
                        </template>
                        <div v-for="(err, index) in sortResult.error" :key="index" style="display: block; margin-bottom: 8px;">
                            <ErrorInfo :err="err" :sequence="index + 1" />
                        </div>
                    </n-popover>
                    <n-tag type="warning" v-if="sortResult.warning.length > 0">
                        <template #icon>
                            <n-icon><WarningAmberFilled /></n-icon>
                        </template>
                        {{ sortResult.warning.length }}
                    </n-tag>
                    <n-tag type="success" v-if="sortResult.error.length === 0 && sortResult.warning.length === 0">
                        <template #icon>
                            <n-icon><CheckOutlined /></n-icon>
                        </template>
                    </n-tag>
                </div>
            </n-flex>
        </n-card>
        <EditUserOrder ref="editUserOrder"></EditUserOrder>
    </div>
</template>

<style scoped>

</style>
