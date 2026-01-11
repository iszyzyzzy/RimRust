<script setup lang="ts">
import { computed, h, onMounted, ref, watch } from 'vue'
import { DataTableBaseColumn, NButton, NFlex, NIcon, NInput, NPopover, useMessage } from 'naive-ui'
import { useBaseListStore } from '../../utils/store'
import { confirmTranslation, rematchTranslation, tranUserIgnoreAdd, removeTranslation, removeTranslationPack, tranUserIgnoreRemove } from '../../../api/tauriFunc'
import { RefreshSharp, SearchFilled } from '@vicons/material'
import MiniInfoCard from '../../utils/components/MiniInfoCard.vue'

const show = defineModel<boolean>('show')
const baseListStore = useBaseListStore()
const message = useMessage()

interface ShowItem {
    key: string
    keyId: string
    keyName: string
    item: string
    itemName: string
}
const unconfirmsShowList = ref<ShowItem[]>([]);
const calcUnconfirmsShowList = () => {
    let t: ShowItem[] = []

    for (const key in baseListStore.translationModData) {
        let items = baseListStore.translationModData[key]
        if(items.type === 'UnconfirmedMatches') {
            for(const item in items.value) {
                t.push({
                    key: key + "-" + item,
                    keyId: key,
                    keyName: baseListStore.getModById(key)!.displayName,
                    item: items.value[item][0],
                    itemName: baseListStore.getModById(items.value[item][0])!.displayName,
                })
            }
        }
    }
    return t
}
watch(() => baseListStore.translationModData,() => {
    unconfirmsShowList.value = calcUnconfirmsShowList()
})

onMounted(() => {
    unconfirmsShowList.value = calcUnconfirmsShowList()
})

const unconfirmsColumns = ref<DataTableBaseColumn<ShowItem>[]>([
    {
        title: '名称',
        key: 'keyName',
        sorter: 'default',
        filter(value, row) {
            return row.keyName.toLowerCase().includes(value.toString().toLowerCase())
        },
        filterOptionValue: null,
        renderFilterIcon: () => {
            return h(NIcon, null, { default: () => h(SearchFilled) })
        },
        renderFilterMenu: () => {
            return h(
                NFlex,
                { style: { padding: '12px' } },
                {
                    default: () => [
                        h(
                            NInput,
                            {
                                size: 'small',
                                placeholder: '搜索',
                                clearable: true,
                                style: { width: '100%' },
                                onUpdateValue: (value: string) => {
                                    if (value === '') {
                                        unconfirmsColumns.value[0].filterOptionValue = null
                                    } else {
                                        unconfirmsColumns.value[0].filterOptionValue = value
                                    }
                                }
                            }
                        )
                    ]
                }
            )
        }
    },
    {
        title: '汉化包',
        key: 'itemName',
        sorter: 'default',
        filter(value, row) {
            return !!~row.itemName.indexOf(value.toString())
        },
        filterOptionValue: null,
        renderFilterIcon: () => {
            return h(NIcon, null, { default: () => h(SearchFilled) })
        },
        renderFilterMenu: () => {
            return h(
                NFlex,
                { style: { padding: '12px' } },
                {
                    default: () => [
                        h(
                            NInput,
                            {
                                size: 'small',
                                placeholder: '搜索',
                                clearable: true,
                                style: { width: '100%' },
                                onUpdateValue: (value: string) => {
                                    if (value === '') {
                                        unconfirmsColumns.value[1].filterOptionValue = null
                                    } else {
                                        unconfirmsColumns.value[1].filterOptionValue = value
                                    }
                                }
                            }
                        )
                    ]
                }
            )
        }
    },
    {
        title: '操作',
        key: 'action',
        render(row: { keyId: string, item: string }) {
            return h(
                NFlex,
                {
                    align: 'center',
                    style: {
                        gap: '10px',
                    },
                },
                {
                    default: () => [
                        h(
                            NButton,
                            {
                                strong: true,
                                tertiary: true,
                                size: 'small',
                                onClick: () => handleView(row.keyId, row.item),
                            },
                            { default: () => '查看' }
                        ),
                        h(
                            NButton,
                            {
                                strong: true,
                                tertiary: true,
                                size: 'small',
                                onClick: () => handleConfirm(row.keyId, row.item),
                            },
                            { default: () => '确认' }
                        ),
                        h(
                            NButton,
                            {
                                strong: true,
                                tertiary: true,
                                size: 'small',
                                onClick: () => handleIgnore(row.keyId),
                            },
                            { default: () => '忽略', }
                        ),
                    ],
                }
            )
        }
    }
])

const emit = defineEmits<{
    view: [id: string, targetId: string]
}>()

const handleConfirm = (id: string, targetId: string) => {
    confirmTranslation(id, targetId).then(() => {
        message.success('确认成功')
    })
    baseListStore.$patch((state) => {
        state.translationModData[id] = { type: 'Matched', value: targetId }
    })
    //console.log('confirm', id, targetId)
}

const handleIgnore = (id: string) => {
    tranUserIgnoreAdd(id).then(() => {
        message.success('忽略成功')
    })
    baseListStore.$patch((state) => {
        state.translationModData[id] = { type: 'Ignored', value: null }
    })
    //console.log('ignore', id, targetId)
}

const handleView = (id: string, targetId: string) => {
    console.log('view', id, targetId)
    emit('view', id, targetId)
}

const loading = ref(false)

const handleRefrush = () => {
    loading.value = true
    rematchTranslation().then((res) => {
        baseListStore.$patch((state) => {
            for (const key in res) {
                state.translationModData[key] = { type: 'UnconfirmedMatches', value: res[key] }
            }
        })
        loading.value = false
    })
}

const confirmsShowList = computed(() => {
    let t = []
    for (const key in baseListStore.translationModData) {
        if (baseListStore.translationModData[key].type === 'Matched') {
            t.push({
                key: key,
                keyId: key,
                keyName: baseListStore.getModById(key)?.displayName,
                item: baseListStore.translationModData[key].value,
                itemName: baseListStore.getModById(baseListStore.translationModData[key].value)?.displayName,
            })
        }
    }
    return t
})

const confirmsColumns = ref<DataTableBaseColumn<ShowItem>[]>([
    {
        title: '名称',
        key: 'keyName',
        sorter: 'default',
        filter(value, row) {
            return row.keyName.toLowerCase().includes(value.toString().toLowerCase())
        },
        filterOptionValue: null,
        renderFilterIcon: () => {
            return h(NIcon, null, { default: () => h(SearchFilled) })
        },
        renderFilterMenu: () => {
            return h(
                NFlex,
                { style: { padding: '12px' } },
                {
                    default: () => [
                        h(
                            NInput,
                            {
                                size: 'small',
                                placeholder: '搜索',
                                clearable: true,
                                style: { width: '100%' },
                                onUpdateValue: (value: string) => {
                                    if (value === '') {
                                        confirmsColumns.value[0].filterOptionValue = null
                                    } else {
                                        confirmsColumns.value[0].filterOptionValue = value
                                    }
                                }
                            }
                        )
                    ]
                }
            )
        }
    },
    {
        title: '汉化包',
        key: 'itemName',
        sorter: 'default',
        filter(value, row) {
            return !!~row.itemName.indexOf(value.toString())
        },
        filterOptionValue: null,
        renderFilterIcon: () => {
            return h(NIcon, null, { default: () => h(SearchFilled) })
        },
        renderFilterMenu: () => {
            return h(
                NFlex,
                { style: { padding: '12px' } },
                {
                    default: () => [
                        h(
                            NInput,
                            {
                                size: 'small',
                                placeholder: '搜索',
                                clearable: true,
                                style: { width: '100%' },
                                onUpdateValue: (value: string) => {
                                    if (value === '') {
                                        confirmsColumns.value[1].filterOptionValue = null
                                    } else {
                                        confirmsColumns.value[1].filterOptionValue = value
                                    }
                                }
                            }
                        )
                    ]
                }
            )
        }
    },
    {
        title: '操作',
        key: 'action',
        render(row: { key: string, item: string }) {
            return h(
                NFlex,
                {
                    align: 'center',
                    style: {
                        gap: '10px',
                    },
                },
                {
                    default: () => [
                        h(
                            NButton,
                            {
                                strong: true,
                                tertiary: true,
                                size: 'small',
                                onClick: () => handleView(row.key, row.item),
                            },
                            { default: () => '查看' }
                        ),
                        h(
                            NButton,
                            {
                                strong: true,
                                tertiary: true,
                                size: 'small',
                                onClick: () => handleRemove(row.key),
                            },
                            { default: () => '删除', }
                        ),
                    ],
                }
            )
        }
    }
])

const handleRemove = (id: string) => {
    removeTranslation(id).then(() => {
        message.success('删除成功')
    })
    delete baseListStore.translationModData[id]
    //console.log('remove', id)
}

const transPackShowList = computed(() => {
    let t = []
    for (const key in baseListStore.translationModData) {
        if (baseListStore.translationModData[key].type === 'Translation') {
            t.push({
                key: key,
                keyId: key,
                keyName: baseListStore.getModById(key)?.displayName,
                item: "",
                itemName: "",
            })
        }
    }
    return t
})

const transPackColumns = ref<DataTableBaseColumn<ShowItem>[]>([
    {
        title: '名称',
        key: 'keyName',
        sorter: 'default',
        filter(value, row) {
            return row.keyName.toLowerCase().includes(value.toString().toLowerCase())
        },
        filterOptionValue: null,
        renderFilterIcon: () => {
            return h(NIcon, null, { default: () => h(SearchFilled) })
        },
        renderFilterMenu: () => {
            return h(
                NFlex,
                { style: { padding: '12px' } },
                {
                    default: () => [
                        h(
                            NInput,
                            {
                                size: 'small',
                                placeholder: '搜索',
                                clearable: true,
                                style: { width: '100%' },
                                onUpdateValue: (value: string) => {
                                    if (value === '') {
                                        transPackColumns.value[0].filterOptionValue = null
                                    } else {
                                        transPackColumns.value[0].filterOptionValue = value
                                    }
                                }
                            }
                        )
                    ]
                }
            )
        }
    },
    {
        title: '操作',
        key: 'action',
        render(row: { key: string, item: string }) {
            return h(
                NFlex,
                {
                    align: 'center',
                    style:
                    {
                        gap: '10px',
                    },
                },
                {
                    default: () => [
                        h(
                            NButton,
                            {
                                strong: true,
                                tertiary: true,
                                size: 'small',
                                onClick: () => handleTransPackRemove(row.key),
                            },
                            { default: () => '删除', }
                        ),
                        h(
                            NPopover,
                            {
                                trigger: 'click',
                                placement: 'bottom',
                                contentStyle: { padding: '0' },
                            },
                            {
                                default: () => [
                                    h(
                                        MiniInfoCard,
                                        { data: baseListStore.getModById(row.key), showDetail: true, maxHeight: 400 },
                                    ),
                                ],
                                trigger: () => h(
                                    NButton,
                                    {
                                        strong: true,
                                        tertiary: true,
                                        size: 'small',
                                    },
                                    { default: () => '查看', }
                                )
                            }
                        )
                    ],
                }
            )
        }
    }
])

const handleTransPackRemove = (id: string) => {
    removeTranslationPack(id).then(() => {
        message.success('删除成功')
    })
    baseListStore.$patch((state) => {
        if(state.translationModData[id].type === 'Translation') {
            delete state.translationModData[id]
        }
    })
    //console.log('remove', id)
}

const userIgnoreShowList = computed(() => {
    let t = []
    for (const key in baseListStore.translationModData) {
        if (baseListStore.translationModData[key].type === 'Ignored') {
            t.push({
                key: key,
                keyId: key,
                keyName: baseListStore.getModById(key)?.displayName,
                item: "",
                itemName: "",
            })
        }
    }
    return t
})

const userIgnoreColumns = ref<DataTableBaseColumn<ShowItem>[]>([
    {
        title: '名称',
        key: 'keyName',
        sorter: 'default',
        filter(value, row) {
            return row.keyName.toLowerCase().includes(value.toString().toLowerCase())
        },
        filterOptionValue: null,
        renderFilterIcon: () => {
            return h(NIcon, null, { default: () => h(SearchFilled) })
        },
        renderFilterMenu: () => {
            return h(
                NFlex,
                { style: { padding: '12px' } },
                {
                    default: () => [
                        h(
                            NInput,
                            {
                                size: 'small',
                                placeholder: '搜索',
                                clearable: true,
                                style: { width: '100%' },
                                onUpdateValue: (value: string) => {
                                    if (value === '') {
                                        userIgnoreColumns.value[0].filterOptionValue = null
                                    } else {
                                        userIgnoreColumns.value[0].filterOptionValue = value
                                    }
                                }
                            }
                        )
                    ]
                }
            )
        }
    },
    {
        title: '操作',
        key: 'action',
        render(row: { key: string }) {
            return h(
                NFlex,
                {
                    align: 'center',
                    style:
                    {
                        gap: '10px',
                    },
                },
                {
                    default: () => [
                        h(
                            NButton,
                            {
                                strong: true,
                                tertiary: true,
                                size: 'small',
                                onClick: () => handleUserIgnoreRemove(row.key),
                            },
                            { default: () => '删除', }
                        ),
                    ],
                }
            )
        }
    }
])

const handleUserIgnoreRemove = (id: string) => {
    tranUserIgnoreRemove(id).then(() => {
        message.success('删除成功')
    })
    baseListStore.$patch((state) => {
        if(state.translationModData[id].type === 'Ignored') {
            delete state.translationModData[id]
        }
    })
    //console.log('remove', id)
}

const unconfirmedCount = computed(() => {
    return Object.keys(baseListStore.translationModData).filter((key) => baseListStore.translationModData[key].type === 'UnconfirmedMatches').length
})

const confirmedCount = computed(() => {
    return Object.keys(baseListStore.translationModData).filter((key) => baseListStore.translationModData[key].type === 'Matched').length
})

</script>

<template>
    <n-modal v-model:show="show">
        <n-card style="width: 800px; height: 600px;" :bordered="false" aria-modal="true" content-style="padding: 0;">
            <n-tabs type="line" size="large" :tabs-padding="20" pane-style="padding: 20px;">
                <n-tab-pane name="未确认">
                    <n-spin :show="loading">
                        <n-data-table :columns="unconfirmsColumns" :data="unconfirmsShowList" virtual-scroll
                            :max-height="450" style="max-width: 750px;">
                        </n-data-table>
                    </n-spin>
                    <template #tab>
                        {{ `未确认[${unconfirmedCount}]` }}
                    </template>
                </n-tab-pane>
                <n-tab-pane name="确认">
                    <n-spin :show="loading">
                        <n-data-table :columns="confirmsColumns" :data="confirmsShowList" virtual-scroll
                            :max-height="450" style="max-width: 750px;">
                        </n-data-table>
                    </n-spin>
                    <template #tab>
                        {{ `确认[${confirmedCount}]` }}
                    </template>
                </n-tab-pane>
                <n-tab-pane name="汉化包">
                    <n-spin :show="loading">
                        <n-data-table :columns="transPackColumns" :data="transPackShowList" virtual-scroll
                            :max-height="450" style="max-width: 750px;">
                        </n-data-table>
                    </n-spin>
                </n-tab-pane>
                <n-tab-pane name="忽略">
                    <n-spin :show="loading">
                        <n-data-table :columns="userIgnoreColumns" :data="userIgnoreShowList" virtual-scroll
                            :max-height="450" style="max-width: 750px;">
                        </n-data-table>
                    </n-spin>
                </n-tab-pane>
                <template #suffix>
                    <n-tooltip trigger="hover">
                        <template #trigger>
                            <n-button size="small" @click="handleRefrush" style="margin: 5px;" :disabled="loading">
                                <n-icon>
                                    <RefreshSharp />
                                </n-icon>
                            </n-button>
                        </template>
                        <span>刷新&尝试识别</span>
                    </n-tooltip>
                </template>
            </n-tabs>
        </n-card>
    </n-modal>
</template>