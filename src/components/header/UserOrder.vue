<script setup lang="ts">
import { computed, h, ref } from 'vue';
import { ModOrder, PackageId } from '../../api/types';
import { NButton, NFlex } from 'naive-ui';

const show = defineModel<boolean>('show')

const data = ref<Record<PackageId, ModOrder[]>>({})

interface showDataItem {
    packageId: PackageId
    order: ModOrder[]
}

const showData = computed(() => {
    return Object.entries(data.value).map(([packageId, order]) => {
        return {
            packageId,
            order
        } as showDataItem
    })
})

const orderToString = (order: ModOrder) => {
    switch (true) {
        case 'Before' in order:
            return `早于 ${order.Before}`
        case 'After' in order:
            return `晚于 ${order.After}`
        case 'First' in order:
            return '最前'
        case 'Last' in order:
            return '最后'
    }
}

const columns = ref([
    {
        title: '包名',
        key: 'packageId',
    },
    {
        title: '顺序',
        key: 'order',
        render(row: showDataItem) {
            return row.order.reduce((acc, cur) => {
                return acc + orderToString(cur) + '\n'
            }, '')
        }
    },
    {
        title: '操作',
        key: 'action',
        render(row: showDataItem) {
            h(
                NFlex,
                {
                    align: 'center',
                    justify: 'center'
                },
                [
                    h(
                        NButton,
                        {
                            onClick: () => {
                                console.log('edit', row)
                            }
                        },
                        {
                            default: () => '编辑'
                        }
                    ),
                    h(
                        NButton,
                        {
                            onClick: () => {
                                console.log('delete', row)
                            }
                        },
                        {
                            default: () => '删除'
                        }
                    )
                ]
            )
        }
    }
])
</script>

<template>
    <n-modal v-model:show="show" preset="card" size="medium" style="max-width: 600px;" title="自定义排序">
        <n-data-table
            :columns="columns"
            :data="showData"
        ></n-data-table>
        <template #footer>
        尾部
        </template>
    </n-modal>
</template>