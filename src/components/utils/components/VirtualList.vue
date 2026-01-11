<script lang="ts" setup>
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { VirtualListProps, DragEndPayload, DragCrossListPayload } from "./VirtualListInterface"
import { useDragStore } from "../store";

import VirtualListItem from "./VirtualListItem.vue";


const {
  items,
  estimatedItemHeight = 30,
  keyField,
  overscan = 5,
  dataListId,
  style: bodyStyle,
} = defineProps<VirtualListProps>();
type ItemType = typeof items[number];

const emit = defineEmits<{
  dragEnd: [payload: DragEndPayload];
  dragCrossList: [payload: DragCrossListPayload]
}>();

interface visibleItem {
  item: ItemType;
  isIndicator: boolean;
}

const listContainer = ref<HTMLElement | null>(null);
const currentTopIndex = ref(0);
const startIndex = ref(0);
const endIndex = ref(0);
const visibleItems = ref<visibleItem[]>([]);
const isScrolling = ref(false);
let scrollTimer: number | null = null;

const itemPositions = ref<{ top: number; height: number }[]>([]);

const initPositions = () => {
  itemPositions.value = items.map((_, index) => ({
    top: index * estimatedItemHeight,
    height: estimatedItemHeight,
  }));
};

const updateItemPosition = (index: number, height: number) => {
  if (index < 0 || index >= itemPositions.value.length) return;
  if (itemPositions.value[index].height === height) return;

  const deltaHeight = height - itemPositions.value[index].height;
  itemPositions.value[index].height = height;
  for (let i = index + 1; i < itemPositions.value.length; i++) {
    itemPositions.value[i].top += deltaHeight;
  }
};

/// containerHeight实际上是在外部写的css表达式
/// 读那个div的高度就行了
const watchResize = (entries: ResizeObserverEntry[]) => {
  for (const entry of entries) {
    if (entry.target === listContainer.value) {
      containerHeight.value = entry.contentRect.height;
      reCalculateIndex();
      updateVisibleItems();
    }
  }
}


const containerHeight = ref(200);

const spacerHeight = computed(() => {
  if (itemPositions.value.length === 0) return 0;
  const lastItem = itemPositions.value[itemPositions.value.length - 1];
  return lastItem.top + lastItem.height; // 总高度
});

const dragState = useDragStore();
const isMouseIn = ref(false);

const updateVisibleItems = () => {
  //const startTime = performance.now();
  const itemsSlice = items.slice(startIndex.value, endIndex.value);

  visibleItems.value = itemsSlice.map((item) => ({
    item,
    isIndicator: false
  }));

  // 如果在drag的话，在对应位置插入一个indicator
  if (isMouseIn.value && dragState.state && dragState.state.dragging && dragState.state.item
      && dragState.state.indicatorType !== 'hide'
  ) {
    // 1. 计算鼠标在整个可滚动内容中的Y坐标
    const mouseYInContent = dragState.state.mouseY - listContainer.value!.getBoundingClientRect().top + scrollTop.value;

    
    // 2. 计算在完整数据源 (items) 中的目标索引
    const globalInsertIndex = itemPositions.value.findIndex(pos => mouseYInContent < pos.top + pos.height);


    // 3. 转换为在当前可见项 (visibleItems) 中的本地索引
    const localInsertIndex = globalInsertIndex - startIndex.value;

    const indicatorItem: visibleItem = {
      item: dragState.state.item,
      isIndicator: true
    };

    const InsertIndex = Math.max(0, Math.min(itemsSlice.length, localInsertIndex));

    // 4. 在本地索引处插入指示器，并确保索引在有效范围内
    visibleItems.value.splice(InsertIndex, 0, indicatorItem);
    //console.log('updatePosition', prevItemKey, nextItemKey);
    dragState.updateTarget({
      itemType: 'mod',
      itemKey: dragState.state.item[keyField],
      fromIndex: dragState.state.startIndex,
      toIndex: globalInsertIndex - (globalInsertIndex > dragState.state.startIndex ? 1 : 0), // 如果是向后拖动，目标索引要减1
    });
  }
  //const endTime = performance.now();
  //console.log(`updateVisibleItems took ${endTime - startTime} ms`);
  //console.log(visibleItems.value)
};

// const reCalculateIndex = () => {
//   currentTopIndex.value = Math.floor(scrollTop.value / itemHeight);
//   startIndex.value = Math.max(0, currentTopIndex.value - overscan);

//   endIndex.value = Math.min(
//     currentTopIndex.value + Math.ceil(containerHeight.value / itemHeight),
//     items.length
//   );
//   endIndex.value = Math.min(
//     endIndex.value + overscan,
//     items.length
//   );
// /*   console.log(
//     `startIndex: ${startIndex.value}, endIndex: ${endIndex.value}, currentTopIndex: ${currentTopIndex.value}`
//   ); */
// };
const reCalculateIndex = () => {
  const list = itemPositions.value;
  if (list.length === 0) {
    startIndex.value = 0;
    endIndex.value = 0;
    currentTopIndex.value = 0;
    return;
  }

  // 二分查找当前顶部可见项的索引
  let low = 0;
  let high = list.length - 1;
  let mid = 0;
  while (low <= high) {
    mid = Math.floor((low + high) / 2);
    if (list[mid].top + list[mid].height < scrollTop.value) {
      low = mid + 1;
    } else if (list[mid].top > scrollTop.value) {
      high = mid - 1;
    } else {
      low = mid;
      break; // 找到精确位置
    }
  }
  currentTopIndex.value = low > 0 ? low - 1 : 0;
  startIndex.value = Math.max(0, currentTopIndex.value - overscan);

  // 计算endIndex
  let visibleHeight = 0;
  let i = startIndex.value;
  while (i < list.length && visibleHeight < containerHeight.value) {
    visibleHeight += list[i].height;
    i++;
  }
  endIndex.value = Math.min(i + overscan * 2, list.length);// 因为start减了一个overscan所以这里*2
}

const scrollTop = ref(0);

const onScroll = () => {
  if (!listContainer.value) return;
  
  isScrolling.value = true;
  if (scrollTimer) {
    clearTimeout(scrollTimer);
  }
  scrollTimer = window.setTimeout(() => {
    isScrolling.value = false;
  }, 150);

  scrollTop.value = listContainer.value?.scrollTop || 0;
  reCalculateIndex();
  updateVisibleItems();
  if (dragState.state && dragState.state.isMousedown) {
    dragState.state.dragging = true; // 按下鼠标不动光滚滚轮也是一种拖拽
  }
  //console.log('scrollTop', scrollTop.value)
};

watch(
  () => items,
  (newValue, oldValue) => {
    const heightCache = new Map<ItemType[typeof keyField], number>();
    if (oldValue && oldValue.length > 0 && itemPositions.value.length === oldValue.length) {
      oldValue.forEach((item, index) => {
        heightCache.set(item[keyField], itemPositions.value[index].height);
      });
    }
    let currentTop = 0;
    itemPositions.value = newValue.map(item => {
      const height = heightCache.get(item[keyField]) || estimatedItemHeight;
      const position = { top: currentTop, height };
      currentTop += height;
      return position;
    });

    reCalculateIndex();
    updateVisibleItems();
  },
  { immediate: true, deep: false }
);


onMounted(() => {
  initPositions();
  updateVisibleItems();
  const resizeObserver = new ResizeObserver(watchResize);

  if (listContainer.value) {
    resizeObserver.observe(listContainer.value);
  }

  onUnmounted(() => {
    resizeObserver.disconnect();
  });
});

const slots = defineSlots<{
  default(props: { item: ItemType, textColor?: string}): any
}>();

const onItemMouseDown = (e: MouseEvent, item: ItemType, index: number) => {
  //console.log('onItemMouseDown', item, index);
  dragState.startDrag(item, dataListId || 0, index, e);

  window.addEventListener("mousemove", onItemMouseMove);
  window.addEventListener("mouseup", onItemMouseUp);
}

const onItemMouseMove = (e: MouseEvent) => {
  //console.log('onItemMouseMove', dragState.value);
  dragState.updateMousePosition(e);
  updateVisibleItems();
  //console.log('cursor type', dragState.cursorStyle.cursor);
  document.body.style.cursor = dragState.cursorStyle.cursor;
};

/* dragState.$subscribe((_, state) => {
  if (isMouseIn.value && state.state?.dragging) {
    updateVisibleItems();
  }
}); */

const onItemMouseUp = (e: MouseEvent) => {
  //console.log('onItemMouseUp', dragState.value);
  window.removeEventListener("mousemove", onItemMouseMove);
  window.removeEventListener("mouseup", onItemMouseUp);
  if (dragState.state && dragState.state.dragging) {
    e.preventDefault();
    e.stopPropagation();
    document.body.classList.remove('dragging');
    document.body.style.cursor = 'default';

    /* if (dragState.state.prevItemKey === null && dragState.state.nextItemKey === null) {
      console.warn('double null on drag end', dragState.state);
    } else  */
    if (dragState.state.currentListId === null) {
      console.log('dropped outside any list, do nothing');
    } else if (dragState.state && dragState.state.cursorType === 'no-drop') {
      console.log('drop blocked by strategy');
    } else {
      const payload: DragEndPayload = {
        dragItemKey: dragState.state.item[keyField],
        target: dragState.state.target,
        fromListId: dragState.state.fromListId,
        toListId: dragState.state.currentListId,
      };
      console.log('emit drag end', payload);
      emit('dragEnd', payload);
    }
  }
  dragState.stopDrag();
  updateVisibleItems();
}

const showScrollbar = ref(false);
watch(
  () => isMouseIn.value,
  (newValue) => {
    if (newValue) {
      showScrollbar.value = true;
    } else {
      showScrollbar.value = false;
    }
  },
  { immediate: true }
);
const isDraggingScrollbar = ref(false);
const scrollbarDragStart = ref(0);
const scrollStartTop = ref(0);

const visibleRatio = computed(() => {
  return containerHeight.value / (spacerHeight.value || 1);
});
const scrollbarStyle = computed(() => {
  const maxScrollTop = spacerHeight.value - containerHeight.value;
  const scrollRatio = maxScrollTop > 0 ? scrollTop.value / maxScrollTop : 0;
  const thumbHeight = Math.max(visibleRatio.value * containerHeight.value, 50);
  const maxThumbTop = containerHeight.value - thumbHeight;
  
  return {
    opacity: showScrollbar.value ? 1 : 0,
    pointerEvents: showScrollbar.value ? 'auto' : 'none',
    height: `${thumbHeight}px`,
    transform: `translateY(${scrollRatio * maxThumbTop}px)`
  };
});

const onScrollbarMouseDown = (e: MouseEvent) => {
  e.preventDefault();
  isDraggingScrollbar.value = true;
  scrollbarDragStart.value = e.clientY;
  scrollStartTop.value = scrollTop.value;
  
  window.addEventListener("mousemove", onScrollbarMouseMove);
  window.addEventListener("mouseup", onScrollbarMouseUp);
};

const onScrollbarMouseMove = (e: MouseEvent) => {
  if (!isDraggingScrollbar.value || !listContainer.value) return;

  showScrollbar.value = true;
  
  const deltaY = e.clientY - scrollbarDragStart.value;
  const visibleRatio = containerHeight.value / spacerHeight.value || 1;
  const thumbHeight = Math.max(visibleRatio * containerHeight.value, 20);
  const maxThumbTop = containerHeight.value - thumbHeight;
  const maxScrollTop = spacerHeight.value - containerHeight.value;
  
  // 计算滚动距离
  const scrollDelta = (deltaY / maxThumbTop) * maxScrollTop;
  const newScrollTop = Math.max(0, Math.min(scrollStartTop.value + scrollDelta, maxScrollTop));
  
  listContainer.value.scrollTop = newScrollTop;
};

const onScrollbarMouseUp = () => {
  isDraggingScrollbar.value = false;
  window.removeEventListener("mousemove", onScrollbarMouseMove);
  window.removeEventListener("mouseup", onScrollbarMouseUp);
};

const isGhostItem = (id: ItemType[keyof ItemType]) => {
  return dragState.state && dragState.state.item && dragState.state.item[keyField] === id;
}

const onMouseIn = () => {
  //console.log('Mouse entered', dataListId);
  isMouseIn.value = true;
  if (dragState.state) {
    dragState.setCurrentListId(dataListId || 0);
    emit('dragCrossList', {
      dragItemKey: dragState.state.item ? dragState.state.item[keyField] : '',
      fromListId: dragState.state.fromListId,
      toListId: dataListId || 0,
    });
  }
  updateVisibleItems();
};
const onMouseLeave = () => {
  //console.log('Mouse left', dataListId);
  isMouseIn.value = false;
  if (dragState.state) {
    dragState.setCurrentListId(null);
  }
  updateVisibleItems();
};

</script>

<template>
  <div class="virtual-list-container"
    @mouseenter="onMouseIn"
    @mouseleave="onMouseLeave"
  >
    <div class="virtual-list hidden-scrollbar" 
        ref="listContainer" 
        @scroll="onScroll" 
        :style="bodyStyle"
        :class="{ 'is-scrolling': isScrolling }"
      >
      <div class="spacer" :style="{ height: spacerHeight + 'px' }"></div>
      <TransitionGroup
        tag="div"
        class="items-wrapper"
        name="list"
        :css="!isScrolling"
        :style="{ transform: `translateY(${itemPositions[startIndex]?.top}px)` }"
      >
        <template 
          v-for="({ item, isIndicator }, idx) in visibleItems" 
          :key="`${isIndicator ? 'drag-indicator' : item[keyField]}`">
          <VirtualListItem
            v-if="!isIndicator"
            :index="startIndex + idx"
            class="list-item"
            :class="{ 'ghost-item': isGhostItem(item[keyField]) }"
            @resize="updateItemPosition"
            @mousedown="onItemMouseDown($event, item, startIndex + idx)"
          >
            <slot :item="item"></slot>
          </VirtualListItem>
          <!-- 如果指示器紧贴着原项目则不显示 -->
          <div
            v-else-if="visibleItems[idx-1]?.item[keyField] !== item[keyField] && visibleItems[idx+1]?.item[keyField] !== item[keyField]"
            class="list-item indicator-item"
            :style="{ height: `${itemPositions[items.findIndex(i => i[keyField] === item[keyField])]?.height}px` }"
          >
            <!-- <slot :item="item" textColor="var(--primary-color)"></slot> -->
          </div>
        </template>
      </TransitionGroup>
    </div>
    <n-el tag="div" class="scrollbar-rail" v-if="!(visibleRatio >= 1)">
      <n-el tag="div" 
        class="scrollbar-thumb" 
        :style="scrollbarStyle"
        @mousedown="onScrollbarMouseDown"
        ></n-el>
    </n-el>
    <Teleport defer to="#teleported">
      <n-el tag="div" style="border: 1px; border-color: var(--border-color); border-radius: var(--border-radius);"
        v-if="dragState.state && dragState.state.dragging && dragState.state.fromListId === dataListId" 
        class="dragging-item" 
        :style="{ top: `${dragState.state.mouseY}px`, left: `${dragState.state.mouseX}px` }">
          <slot :item="dragState.state.item" textColor="rgba(0,0,0,0.8)" ></slot>
      </n-el>
    </Teleport>
    <Teleport defer to="#teleported">
      <Transition name="fade">
        <div v-if="dragState.state && dragState.state.dragging && dragState.state.tooltip"
        class="fade-item"
        :style="{ top: `${dragState.state.mouseY}px`, left: `${dragState.state.mouseX}px` }">
          {{ dragState.state.tooltip }}
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.virtual-list-container {
  position: relative;
  width: 100%;
  height: 100%;
}
.virtual-list {
  overflow-y: auto;
  overflow-x: hidden;
  position: relative;
  width: 100%;
}

.hidden-scrollbar {
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.hidden-scrollbar::-webkit-scrollbar {
  display: none;
}
.scrollbar-rail {
  position: absolute;
  right: 0;
  top: 0;
  height: 100%;
  width: var(--scrollbar-width);
  background-color: transparent;
}
.scrollbar-thumb {
  position: relative;
  background: var(--scrollbar-color);
  border-radius: var(--scrollbar-border-radius);
  transition: background-color .2s var(--cubic-bezier-ease-in-out),
              opacity .2s;
  width: var(--scrollbar-width);
  cursor: pointer;
}
.scrollbar-thumb:hover {
  background: var(--scrollbar-color-hover);
}
body.dragging .scrollbar-thumb {
  background: var(--scrollbar-color)
}

.spacer {
  width: 100%;
  pointer-events: none;
}

.items-wrapper {
  position: absolute;
  left: 0;
  top: 0;
  width: 100%;
}

.list-move,
.list-enter-active,
.list-leave-active {
  transition: transform 0.12s ease,
              background-color 0.2s ease,
              color 0.2s ease,
              opacity 0.07s ease;
}

.list-enter-from,
.list-leave-to {
  opacity: 0;
  transform: translateX(30px);
}

.list-leave-active {
  position: absolute !important;
  z-index: -10;
}

.list-item {
  position: relative;
  left: 0;
  width: 100%;
  box-sizing: border-box;
}
.ghost-item {
  opacity: 0.5;
}
.dragging-item {
  position: fixed;
  z-index: 1000;
  pointer-events: none;
  transform: translate(-25%, -50%);
  user-select: none;
  background-color: rgba(200,200,200,0.8);
  color: rgba(0,0,0,0.8) !important;
}

.fade-item {
  position: fixed;
  z-index: 1000;
  pointer-events: none;
  user-select: none;
  overflow: hidden;
  text-wrap: nowrap;
  transform: translate(-25%, -200%);
  background-color: rgba(200,200,200,0.8);
  color: rgba(0,0,0,0.8) !important;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.5s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
