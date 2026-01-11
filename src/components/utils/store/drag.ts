import { defineStore } from 'pinia';
import { DragState, DragTargetFlat, DragTargetGroup } from '@utils/components/VirtualListInterface';
import { computed, ref } from 'vue';
import deleteCursor from '@assets/deleteCursor.svg';

export const useDragStore = defineStore('drag', () => {
    const state = ref<DragState | null>(null);

    function startDrag(item: Record<string, any>, listId: number, startIndex: number, event: MouseEvent) {
        state.value = {
            dragging: false,
            isMousedown: true,
            startX: event.clientX,
            startY: event.clientY,
            mouseX: event.clientX,
            mouseY: event.clientY,
            item: item,
            startIndex,
            fromListId: listId,
            currentListId: listId,
            target: null,
            indicatorType: 'normal',
            cursorType: 'move',
            tooltip: null,
        }
    }

    function stopDrag() {
        if (state.value) {
            state.value = null
            setDragInfo(null);
        }
    }

    function updateMousePosition(event: MouseEvent) {
        if (state.value && state.value.isMousedown) {
            const dx = event.clientX - state.value.startX;
            const dy = event.clientY - state.value.startY;

            if (!state.value.dragging && (Math.abs(dx) >= 5 || Math.abs(dy) >= 5)) {
                state.value.dragging = true;
                document.body.classList.add('dragging');
            }

            if (state.value.dragging) {
                state.value.mouseX = event.clientX;
                state.value.mouseY = event.clientY;
            }
        }
    }

    function updateTarget(target: DragTargetFlat | DragTargetGroup | null) {
        if (state.value) {
            state.value.target = target;
        }
    }

    function setCurrentListId(listId: number | null) {
        if (state.value) {
            state.value.currentListId = listId;
            state.value.target = null;
        }
    }

    function setDragInfo(info?: Partial<Pick<DragState, 'indicatorType' | 'cursorType' | 'tooltip'>> | null) {
        if (state.value) {
            if (info === undefined) return;
            if (info === null) {
                state.value.indicatorType = 'normal';
                state.value.cursorType = 'default';
                state.value.tooltip = null;
                return;
            }
            if (info.indicatorType !== undefined) state.value.indicatorType = info.indicatorType;
            if (info.cursorType !== undefined) state.value.cursorType = info.cursorType;
            if (info.tooltip !== undefined) state.value.tooltip = info.tooltip;
        }
    }

    const cursorStyle = computed(() => {
        let cursor: string = state.value && state.value.cursorType ? state.value.cursorType : 'default';
        if (cursor === 'delete') {
            cursor = `url("${deleteCursor}") 8 8, auto`;
        }
        return {
            cursor: cursor
        };
    });

    return { state, startDrag, stopDrag, updateMousePosition, setCurrentListId, updateTarget, setDragInfo, cursorStyle }
});

export type DragStore = ReturnType<typeof useDragStore>;