<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';

const props = defineProps<{ index: number }>();
const emit = defineEmits<{ (e: 'resize', index: number, height: number): void }>();

const itemRef = ref<HTMLElement | null>(null);
let resizeObserver: ResizeObserver;

onMounted(() => {
  if (itemRef.value) {
    resizeObserver = new ResizeObserver(([entry]) => {
      emit('resize', props.index, entry.contentRect.height);
    });
    resizeObserver.observe(itemRef.value);
  }
});

onUnmounted(() => {
  if (resizeObserver) {
    resizeObserver.disconnect();
  }
});
</script>

<template>
  <div ref="itemRef">
    <slot></slot>
  </div>
</template>