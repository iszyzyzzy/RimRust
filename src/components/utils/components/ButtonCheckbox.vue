<script setup lang="ts">
import { ref } from 'vue';
import { CheckRound } from '@vicons/material';

const { text, handleClick } = defineProps<{
    text: string
    handleClick: (value: boolean) => void
}>()

const state = defineModel<boolean>({ default: false })

</script>

<template>
    <n-button tertiary :type="state ? 'success' : 'default'" @click="handleClick(state = !state)">
        <template v-if="state" #icon>
            <n-icon>
                <CheckRound />
            </n-icon>
        </template>
        {{ text }}
    </n-button>
</template>