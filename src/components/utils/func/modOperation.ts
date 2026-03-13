import { Id, ModChange, ModInner } from '@api/types';
import { setEnableMod as apiSetEnableMod, resetModOrder, changeModDisplayOrder } from '@api/tauriFunc';
import { BaseListStore } from '@store';

export const isMod = (item: ModInner): item is ModInner => {
    return 'displayName' in item
}

const checkMods = (mods: (ModInner | null)[]): mods is ModInner[] => {
    if (mods.some(mod => mod === null)) {
        return false;
    }
    return true;
}

export const setEnableMod = (modIds: Id[], enabled: boolean, baseListStore: BaseListStore) => {
    const mods = modIds.map(id => baseListStore.getModById(id));

    if (!checkMods(mods)) {
        throw new Error('Invalid mod IDs provided');
    }

    const modChanges:[Id, ModChange[]][] = mods.map(mod => {
        return [mod.id, [{enabled}]];
    });

    console.log('setEnableMod', modIds, enabled, modChanges);
    console.trace();

    baseListStore.handleModChanges(modChanges);

    return apiSetEnableMod(modIds, enabled)
}


export const changeModOrder = async (modId: Id, fromIndex: number, toIndex: number, baseListStore: BaseListStore): Promise<void> => {
    console.log('changeModOrder', modId, fromIndex, toIndex);
    const mod = baseListStore.getModById(modId);
    if (!mod) {
        throw new Error('Invalid mod ID provided');
    }
    if (fromIndex === -1 || toIndex === -1) {
        return;
    }
    baseListStore.$patch(state => {
        state.modsOrder.splice(fromIndex, 1);
        state.modsOrder.splice(toIndex, 0, mod.id);
    })
    return changeModDisplayOrder(fromIndex, toIndex);
}