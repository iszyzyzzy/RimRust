import { Id } from "@/api/types";


export interface VirtualListProps<T = any> {
  items: T[]; // 列表数据
  estimatedItemHeight: number; // 预估每项高度
  keyField: keyof T; // 唯一键字段
  overscan?: number; // 预渲染项数量
  dataListId?: number; // 列表ID，用于跨列表拖拽
  style?: Record<string, string>; // 自定义样式
}

// export type ItemType = VirtualListProps extends VirtualListProps<infer U> ? U : any;

export interface VirtualListInst<T = any> {
  scrollToIndex: (index: number) => void;
  scrollToKey: (key: T[keyof T]) => void;
}

export interface DragEndPayload<T = any> {
  dragItemKey: Id; // 被拖拽项的键值
  fromListId: number; // 当前列表ID
  toListId: number; // 目标列表ID
  target: DragTargetFlat<T> | DragTargetGroup<T> | null; // 放置目标
}

export interface DragCrossListPayload {
  dragItemKey: Id;
  fromListId: number;
  toListId: number;
}

export interface DragTargetFlat<T = any> {
  itemType: "mod" | "group";
  itemKey: Id;
  fromIndex: number;
  toIndex: number;
}

export interface DragTargetGroup<T = any> {
  GroupKey: string;
  place: DragTargetFlat<T> | DragTargetGroup<T>;
  inst: any; // TODO 指向当前实例的
}

// TODO NNEXT
export interface DragState<T = any> {
  dragging: boolean;
  isMousedown: boolean;
  startX: number;
  startY: number;
  mouseX: number;
  mouseY: number;
  startIndex: number;
  fromListId: number;
  currentListId: number | null; // null表示不在任何列表内
  target: DragTargetFlat<T> | DragTargetGroup<T> | null;
  item: T;
  indicatorType: 'normal' | 'hide';
  cursorType: 'move' | 'copy' | 'delete' | 'no-drop' | 'default';
  tooltip: string | null;
}