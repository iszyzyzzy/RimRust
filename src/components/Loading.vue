<script setup lang="ts">

const props = defineProps({
    show: Boolean,
    context: String,
})


</script>
<template>
    <n-modal :show="props.show" title="Loading" size="huge">
        <n-card>
        <n-result title="正在加载" :description="props.context" />
        </n-card>
    </n-modal>
</template>