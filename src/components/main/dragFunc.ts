import { DragEndPayload, DragCrossListPayload } from "../utils/components/VirtualListInterface";
import { changeModOrder, setEnableMod} from "../utils/func/modOperation";
import { DragStore, BaseListStore } from "../utils/store";

const dragEndStrategies: Record<string, (payload: DragEndPayload, dragStore: DragStore, baseListStore: BaseListStore) => void> = {
    "1->1": (payload, _, baseListStore) => {
        if (payload.target === null) {
            console.warn('target is null');
            return;
        }
        if ('GroupKey' in payload.target) {
            console.warn('wrong target type');
            return;
        }
        changeModOrder(payload.dragItemKey, payload.target.fromIndex, payload.target.toIndex, baseListStore);
    },
    "1->3": (payload, _, baseListStore) => {
        setEnableMod([payload.dragItemKey], true, baseListStore);
    },
    "3->1": (payload, _, baseListStore) => {
        setEnableMod([payload.dragItemKey], false, baseListStore);
    },
    "3->3": (payload, dragStore, baseListStore) => {
        // TODO
    }
}


export const handleDragEnd = (payload: DragEndPayload, dragStore: DragStore, baseListStore: BaseListStore) => {
    const { dragItemKey, fromListId, toListId } = payload;
    if (toListId === null) {
        return;
    }
    const strategyKey = `${fromListId}->${toListId}`;
    const strategy = dragEndStrategies[strategyKey];
    console.log('handleDragEnd', payload, strategyKey, strategy);
    if (strategy) {
        strategy(payload, dragStore, baseListStore);
    } else {
        dragStore.stopDrag();
    }
};

// 这个用来处理跨列表拖拽时的鼠标悬停提示
const dragInfoStrategies: Record<string, (dragItemKey: string, dragStore: DragStore, baseListStore: BaseListStore) => void> = {
    "1->2": (dragItemKey, dragStore, baseListStore) => {
        // TODO
    },
    "1->3": (dragItemKey, dragStore, baseListStore) => {
        // 如果拖过去的mod已启用，就不允许拖过去
        const mod = baseListStore.getModById(dragItemKey);
        console.log('handleDragCrossList 1->3', dragItemKey, mod, mod?.enabled);
        if (mod!.enabled) {
            dragStore.setDragInfo({
                indicatorType: 'hide',
                cursorType: 'no-drop',
                tooltip: '该Mod已启用'
            });
        }
    },
    "3->1": (dragItemKey, dragStore, baseListStore) => {
        dragStore.setDragInfo({
            indicatorType: 'hide',
            cursorType: 'delete',
            tooltip: '禁用Mod'
        });
    },
    "3->3": (dragItemKey, dragStore, baseListStore) => {
        dragStore.setDragInfo({
            indicatorType: 'hide',
            cursorType: 'move',
        });
    }
};

export const handleDragCrossList = (payload: DragCrossListPayload,
                                    dragStore: DragStore,
                                    baseListStore: BaseListStore) => {
    const { dragItemKey, fromListId, toListId } = payload;

    if (toListId === null) {
        return;
    }
    const strategyKey = `${fromListId}->${toListId}`;
    const strategy = dragInfoStrategies[strategyKey];
    console.log('handleDragCrossList', payload, strategyKey, strategy);
    if (strategy) {
        strategy(dragItemKey, dragStore, baseListStore);
    } else {
        dragStore.setDragInfo(null);
    }
};