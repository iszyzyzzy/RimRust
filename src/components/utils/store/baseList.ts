import { defineStore } from 'pinia';
import {
    applyModChanges,
    BaseListForSave,
    convertRawModChangeToModChange,
    Id,
    ModChange,
    SyncMessage,
} from '@api/types';


export const useBaseListStore = defineStore('base-list', {
    state: (): BaseListForSave => {
        return {
            mods: {},
            modsOrder: [],
            modsGroups: {},
            modGroupsOrder: [],
            userCustomModsOrder: {},
            userIgnoreInfo: {},
            translationModData: {},
            modNextDisplayOrder: 0,
            groupNextDisplayOrder: 0,
        }
    },
    getters: {
        getModById: (state) => (id: string) => {
            return state.mods[id] || null;
        },
        getModOrderd: (state) => {
            return state.modsOrder.map(id => state.mods[id]);
        },
        getModEnabled: (state) => {
            return state.modsOrder.map(id => state.mods[id]).filter(mod => mod && mod.enabled);
        },
        getTranslation: (state) => (id: Id) => {
            if (!state.translationModData[id]) return null;
            //console.log('getTranslation', id, state.translationModData[id]);
            if (state.translationModData[id].type === 'Matched') {
                return state.mods[state.translationModData[id].value] || null;
            }
            return null;
        }
    },
    actions: {
        handleModChanges(payloads: [Id, ModChange[]][]) {
            for (const [mod_id, changes] of payloads) {
                const mod = this.mods[mod_id];
                if (mod) {
                    this.mods[mod_id] = applyModChanges(mod, changes);
                }
            }
        },
        handleSyncManyPayload(payloads: SyncMessage[]) {
            for (const payload of payloads) {
                if ('ModSync' in payload.operation) {
                    const [mod_id, operation] = payload.operation.ModSync;
                    if ('Add' in operation) {
                        this.mods[mod_id] = operation.Add;
                    } else if ('Update' in operation) {
                        const change = operation.Update.map(convertRawModChangeToModChange);
                        this.mods[mod_id] = applyModChanges(this.mods[mod_id], change);
                    } else if ('Remove' in operation) {
                        delete this.mods[mod_id];
                    } else {
                        let n: never = operation;
                        console.error(n);
                    }
                } else if ('TranslationSync' in payload.operation) {
                    let inner = payload.operation.TranslationSync;
                    if ('AddUnconfirmed' in inner) {
                        let [mod_id, translations] = inner.AddUnconfirmed;
                        this.translationModData[mod_id] = { type: 'UnconfirmedMatches', value: translations };
                    } else if ('RemoveUnconfirmed' in inner) {
                        if ('UnconfirmedMatches' in this.translationModData[inner.RemoveUnconfirmed]) {
                            delete this.translationModData[inner.RemoveUnconfirmed];
                        }
                    } else if ('AddMatch' in inner) {
                        let [source, target] = inner.AddMatch;
                        this.translationModData[source] = { type: 'Matched', value: target };
                    } else if ('RemoveMatch' in inner) {
                        if ('Matched' in this.translationModData[inner.RemoveMatch]) {
                            delete this.translationModData[inner.RemoveMatch];
                        }
                    } else if ('AddUserIgnore' in inner) {
                        this.translationModData[inner.AddUserIgnore] = { type: 'Ignored', value: null };
                    } else if ('RemoveUserIgnore' in inner) {
                        if ('Ignored' in this.translationModData[inner.RemoveUserIgnore]) {
                            delete this.translationModData[inner.RemoveUserIgnore];
                        }
                    } else if ('AddTranPack' in inner) {
                        this.translationModData[inner.AddTranPack] = { type: 'Translation', value: null };
                    } else if ('Remove' in inner) {
                        if (!('Matched' in this.translationModData[inner.Remove])) {
                            delete this.translationModData[inner.Remove];
                        }
                    } else {
                        let n: never = inner;
                        console.error(n);
                    }
                }
            }
        },
    }
})

export type BaseListStore = ReturnType<typeof useBaseListStore>;