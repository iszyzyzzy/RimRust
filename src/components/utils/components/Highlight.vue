<template>
    <div>
        <span :style="{ color: themeVars.textColor3 }" v-if="props.preText">
            {{ props.preText }}
            <br />
            <br />
        </span>
        <n-highlight :text="props.text" :patterns="props.patterns" :highlight-style="{
            padding: '0 1px',
            borderRadius: themeVars.borderRadius,
            display: 'inline-block',
            color: themeVars.baseColor,
            background: themeVars.primaryColor,
            transition: `all .3s ${themeVars.cubicBezierEaseInOut}`,
        }" />
    </div>
</template>

<script setup lang="ts">
import { useThemeVars } from 'naive-ui';

const themeVars = useThemeVars()
const props = defineProps<{
    text: string | null | undefined
    preText?: string | null | undefined
    patterns?: string[] | null | undefined
}>()
</script>