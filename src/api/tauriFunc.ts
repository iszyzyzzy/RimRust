import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { 
  BaseListForSave,
  BaseListForSaveRaw,
  Id,
  ModsGroupForSave,
  SortResult,
  ScanInfoType,
  SearchResult,
  SteamDatabase,
  SyncMessage,
  ScanResult,
  TranslateResponse,
  SaveMetaData,
  ModOrder,
  PackageId,
  CustomCalcResult,
  ModInner
} from './types';
import { convertRawModToMod } from './types';

// 基础列表操作
export async function getBaseList(): Promise<BaseListForSave> {
  let res = await invoke<BaseListForSaveRaw>('base_list_get');

  return {
    mods: res.mods.reduce((acc, mod) => {
      acc[mod.id] = convertRawModToMod(mod);
      return acc;
    }, {} as Record<Id, ModInner>),
    modsOrder: res.modsOrder,
    modsGroups: res.modsGroups.reduce((acc, group) => {
      acc[group.id] = group;
      return acc;
    }, {} as Record<Id, ModsGroupForSave>),
    modGroupsOrder: res.modGroupsOrder,
    userCustomModsOrder: res.userCustomModsOrder,
    userIgnoreInfo: res.userIgnoreInfo,
    translationModData: res.translationModData,
    modNextDisplayOrder: res.modNextDisplayOrder,
    groupNextDisplayOrder: res.groupNextDisplayOrder
  };
}

export async function setEnableMod(modIds: string[], enabled: boolean): Promise<void> {
  return await invoke('mod_set_enable', { modId: modIds, enabled });
}

export async function setModDisplayName(modId: string, newName: string): Promise<void> {
  return await invoke('mod_set_display_name', { modId, newName });
}

export async function changeModDisplayOrder(fromIndex: number, toIndex: number): Promise<void> {
  return await invoke('mod_change_order', { fromIndex, toIndex });
}

export async function resetModOrder(): Promise<[Id, number][]> {
  return await invoke('mod_reset_order');
}

export async function refreshModsData(): Promise<[[Id, number][], [Id, number][], Record<string, Id>]> {
  return await invoke('mod_refresh_all');
}


export async function addModGroup(group: ModsGroupForSave): Promise<void> {
  return await invoke('group_create', { group });
}

export async function removeModGroup(groupId: string): Promise<void> {
  return await invoke('group_delete', { groupId });
}

export async function addObjectToGroup(groupId: string, modId: string): Promise<void> {
  return await invoke('group_add_object', { groupId, modId });
}

export async function removeObjectFromGroup(groupId: string, modId: string): Promise<void> { 
  return await invoke('group_remove_object', { groupId, modId });
}

export async function renameModGroup(groupId: string, newName: string): Promise<void> {
  return await invoke('group_rename', { groupId, newName });
}

export async function setEnableModGroup(groupId: string, enabled: boolean): Promise<void> {
  return await invoke('group_set_enable', { groupId, enabled });
}

export async function changeModsGroupDisplayOrder(groupId: string, newOrder: number): Promise<void> {
  return await invoke('group_change_order', { groupId, newOrder });
}

export async function resetGroupOrder(): Promise<[Id, number][]> {
  return await invoke('group_reset_order');
}



// 获取排序好的模组列表
export async function getSortedMods(): Promise<SortResult> {
  const startTime = performance.now();
  const result = await invoke<SortResult>('sort_mods');
  const endTime = performance.now();
  console.log(`Sort took ${endTime - startTime}ms`, result);
  return result;
}

// 添加用户自定义排序规则
export async function addUserCustomModOrder(packageId: PackageId, order: ModOrder): Promise<void> {
  return await invoke('sort_set_user_custom_order', { packageId, order });
}

// 移除用户自定义排序规则
export async function removeUserCustomModOrder(packageId: PackageId, order: ModOrder): Promise<void> {
  return await invoke('sort_remove_user_custom_order', { packageId, order });
}


export async function loadModsFromXml(): Promise<void> {
  return await invoke('xml_load_file');
}

export async function loadFromGameConfig(): Promise<void> {
  return await invoke('xml_load_from_config');
}

export async function saveModsToXml(mods: Id[]): Promise<void> {
  return await invoke('xml_save_file', { mods });
}

export async function saveToGameConfig(mods: Id[]): Promise<void> {
  return await invoke('xml_save_to_config', { mods });
}


export async function scanErr(): Promise<ScanResult> {
  return await invoke('scan_err');
}

export async function addIgnoreInfo(modId: string, info: ScanInfoType): Promise<void> {
  return await invoke('scan_err_ignore_add', { modId, info });
}

export async function removeIgnoreInfo(modId: string, info: ScanInfoType): Promise<void> {
  return await invoke('scan_err_ignore_remove', { modId, info });
}


export async function getTranslationUnconfirmed(): Promise<Record<Id, [Id, number][]>> {
  return await invoke('tran_unconfirmed_get');
}

export async function confirmTranslation(modId: Id, translationId: Id): Promise<void> {
  return await invoke('tran_comfirm', { modId, translationId });
}

export async function getTranslationMatch(): Promise<Record<Id, Id>> {
  return await invoke('tran_match_get');
}

export async function removeTranslation(modId: Id): Promise<void> {
  return await invoke('tran_remove', { modId });
}

export async function rematchTranslation(): Promise<Record<Id, [Id, number][]>> {
  return await invoke('tran_rematch_all', {});
}

export async function tranUserIgnoreAdd(modId: Id): Promise<void> {
  return await invoke('tran_user_ignore_add', { modId });
}

export async function tranUserIgnoreRemove(modId: Id): Promise<void> {
  return await invoke('tran_user_ignore_remove', { modId });
}

export async function getTranslationPack(): Promise<Id[]> {
  return await invoke('tran_package_get');
}

export async function addTranslationPack(pack: Id): Promise<void> {
  return await invoke('tran_package_add', { pack });
}

export async function removeTranslationPack(pack: Id): Promise<void> {
  return await invoke('tran_package_remove', { pack });
}

export async function translationMatchCustomCalc(sourceId: Id, targetId: Id): Promise<CustomCalcResult> {
  return await invoke('tran_custom_calc', { sourceId, targetId });
}

// 搜索
export async function searchMods(searchText: string, searchField: string[], enabledOnly: boolean = false): Promise<SearchResult> {
  const startTime = performance.now();
  const result = await invoke<SearchResult>('search_mod', { searchText, searchField, enabledOnly });
  const endTime = performance.now();
  console.log(`Search took ${endTime - startTime}ms`, { 
    searchText, 
    searchField, 
    result
  });
  return result;
}
/* 
export async function searchMods(searchText: string, searchField: string[]): Promise<SearchResult> {
  return await invoke('search_mods', { searchText, searchField });
}
 */


// steamDB数据
export async function getSteamDBDataByPackageId(packageId: string): Promise<SteamDatabase[]> {
  return await invoke('steam_db_get_data_by_package_id', { packageId });
}

export async function autoTranslate(text: string,from: string,to: string): Promise<TranslateResponse> {
  return await invoke('translate', { text,from,to });
}

export async function loadSaveMetaData(): Promise<[SaveMetaData | null, SaveMetaData | null]> {
  return await invoke('save_mata_data_get');
}

// 事件监听
export function listenModSync(callback: (msg: { payload: SyncMessage }) => void) {
  /*
  * @deprecated
  */
  return listen('sync-mod', callback);
}

export function listenSyncMany(callback: (msg: { payload: SyncMessage[] }) => void) {
  return listen('sync_many', callback);
}

export function listenStartLoading(callback: (msg: { payload: string }) => void) {
  return listen('start-loading', callback);
}

export function listenEndLoading(callback: () => void) {
  return listen('end-loading', callback);
}

export function findPreviewImage(modPath: string): Promise<string | null> {
  return invoke('find_preview_image', { modPath });
}