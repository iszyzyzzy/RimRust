import { defineStore } from 'pinia';

export interface AppConfig {
    app_data_path: string;
    game_config_path: string;
    steam_mods_path: string;
    game_path: string;
    game_version: string;
    community_rules_update_path: string;
    steam_db_update_path: string;
    prefer_language: string;
    fallback_language: string;
    use_advance_search: boolean;
    proxy: string | null;
}

export const useAppConfigStore = defineStore('app-config', {
    state: (): AppConfig => {
        return {
            app_data_path: '',
            game_config_path: '',
            steam_mods_path: '',
            game_path: '',
            game_version: '',
            community_rules_update_path: '',
            steam_db_update_path: '',
            prefer_language: '',
            fallback_language: '',
            use_advance_search: false,
            proxy: null,
        }
    }
})

export { useBaseListStore, type BaseListStore } from './baseList';


export const useInitdStore = defineStore('initd', {
    state: () => {
        return {
            inited: false,
        }
    },
    actions: {
        async wait_for_init() {
            if (this.inited) return;
            await new Promise(resolve => {
                const interval = setInterval(() => {
                    if (this.inited) {
                        clearInterval(interval);
                        resolve(null);
                    }
                }, 100);
            })
        }
    }
})

export type InitdStore = ReturnType<typeof useInitdStore>;

export { useDragStore, type DragStore } from './drag';