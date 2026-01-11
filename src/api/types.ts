export type Id = string;

export type Version = string;

export type PackageId = string;

export interface ModInner {
  id: Id;
  enabled: boolean;
  visible: boolean;
  packageId: PackageId;
  name: string;
  author: string;
  displayName: string;
  description: VersionMapInterface<string>;
  dependencies: VersionMapInterface<ModDependency[]>;
  supportedVersion: Version[];
  path: string;
  loadOrder: VersionMapInterface<ModOrder[]>;
  incompatibleWith: VersionMapInterface<PackageId[]>;
  supportLanguages: VersionMapInterface<string[]>;
}

export interface ModInnerRaw {
  id: Id;
  enabled: boolean;
  visible: boolean;
  packageId: PackageId;
  name: string;
  author: string;
  displayName: string;
  description: Record<string, string>;
  dependencies: Record<string, ModDependency[]>;
  supportedVersion: Version[];
  path: string;
  loadOrder: Record<string, ModOrder[]>;
  incompatibleWith: Record<string, PackageId[]>;
  supportLanguages: Record<string, string[]>;
}

export function convertRawModToMod(raw: ModInnerRaw): ModInner {
  return {
    ...raw,
    description: VersionMap.fromMap(raw.description),
    dependencies: VersionMap.fromMap(raw.dependencies),
    loadOrder: VersionMap.fromMap(raw.loadOrder),
    incompatibleWith: VersionMap.fromMap(raw.incompatibleWith),
    supportLanguages: VersionMap.fromMap(raw.supportLanguages)
  };
}

export interface ModsGroupForSave {
  id: string;
  name: string;
  enabled: boolean;
  mods: ModsGroupItemForSave[];
  displayOrder: number;
}

export type ModsGroupItemForSave = {
  mod: string;
} | {
  modsGroup: string;
}

export type TranslateModStatus =
  { type: 'Translation', value: null } |
  { type: 'Matched', value: Id } |
  { type: 'UnconfirmedMatches', value: [Id, number][] } |
  { type: 'Ignored', value: null } |
  { type: 'NoMatch', value: null };

export interface BaseListForSave {
  mods: Record<Id, ModInner>;
  modsOrder: Id[];
  modsGroups: Record<Id, ModsGroupForSave>;
  modGroupsOrder: Id[];
  userCustomModsOrder: Record<string, ModOrder[]>;
  userIgnoreInfo: Record<string, ScanInfoType[]>;
  translationModData: Record<string, TranslateModStatus>;
  modNextDisplayOrder: number;
  groupNextDisplayOrder: number;
}

export interface BaseListForSaveRaw {
  mods: ModInnerRaw[];
  modsOrder: Id[];
  modsGroups: ModsGroupForSave[];
  modGroupsOrder: Id[];
  userCustomModsOrder: Record<string, ModOrder[]>;
  userIgnoreInfo: Record<string, ScanInfoType[]>;
  translationModData: Record<string, TranslateModStatus>;
  modNextDisplayOrder: number;
  groupNextDisplayOrder: number;
}

export type ModOrder =
  { type: 'Before', value: PackageId } |
  { type: 'After', value: PackageId } |
  { type: 'First', value: null } |
  { type: 'Last', value: null };

export interface ModDependency {
  package_id: PackageId;
  display_name: string | null;
  url: string | null;
  steam_id: SteamId | null;
  optional: boolean;
}

export type SteamId = {
  AppId: string;
} | {
  WorkshopId: string;
}

export type SyncOperation = {
  ModSync: [Id, ModSyncOperation];
} | {
  TranslationSync: TranslationSyncOperation;
}

export type ModSyncOperation = {
  Add: ModInner;
} | {
  Update: ModChangeRaw[];
} | {
  Remove: Id;
}

export type TranslationSyncOperation = {
  AddUnconfirmed: [Id, [Id, number][]];
} | {
  RemoveUnconfirmed: Id;
} | {
  AddMatch: [Id, Id];
} | {
  RemoveMatch: Id;
} | {
  AddUserIgnore: Id;
} | {
  RemoveUserIgnore: Id;
} | {
  AddTranPack: Id;
} | {
  Remove: Id;
}

export interface SyncMessage {
  id: number;
  operation: SyncOperation;
}

export interface SortResult {
  list: Id[];
  error: SortError[];
  warning: SortWarning[];
  info: string[];
}

export type SortError = {
  CircularDependency: Id[];
} | {
  IncompatibleMods: [Id, Id];
} | {
  MissingDependency: [Id, PackageId, string];
}

export type SortWarning = {
  ConflictingOrders: [Id, Id];
} | {
  DuplicatePackageId: PackageId;
}

export interface SearchResult {
  total: number;
  highlight: string[];
  mods: SearchResultItem[];
}

export interface SearchResultItem {
  id: Id;
  score: number;
  matched_fields: string[];
}

export interface TaskStatus {
  id: string;
  name: string;
  status: string;
  info: string;
  progress: number;
}

export type ModChangeRaw = {
  name: string;
} | {
  author: string;
} | {
  description: Record<string, string>;
} | {
  dependencies: Record<string, ModDependency[]>;
} | {
  supportedVersion: Version[];
} | {
  loadOrder: Record<string, ModOrder[]>;
} | {
  incompatibleWith: Record<string, PackageId[]>;
} | {
  supportLanguages: Record<string, string[]>;
} | {
  enabled: boolean;
} | {
  displayName: string;
}

export function convertRawModChangeToModChange(raw: ModChangeRaw): ModChange {
  if ('description' in raw) {
    return { description: VersionMap.fromMap(raw.description) };
  } else if ('dependencies' in raw) {
    return { dependencies: VersionMap.fromMap(raw.dependencies) };
  } else if ('loadOrder' in raw) {
    return { loadOrder: VersionMap.fromMap(raw.loadOrder) };
  } else if ('incompatibleWith' in raw) {
    return { incompatibleWith: VersionMap.fromMap(raw.incompatibleWith) };
  } else if ('supportLanguages' in raw) {
    return { supportLanguages: VersionMap.fromMap(raw.supportLanguages) };
  } else {
    return raw;
  }
}

export type ModChange = {
  name: string;
} | {
  author: string;
} | {
  description: VersionMap<string>;
} | {
  dependencies: VersionMap<ModDependency[]>;
} | {
  supportedVersion: Version[];
} | {
  loadOrder: VersionMap<ModOrder[]>;
} | {
  incompatibleWith: VersionMap<PackageId[]>;
} | {
  supportLanguages: VersionMap<string[]>;
} | {
  enabled: boolean;
} | {
  displayName: string;
}

type ApplyModChange<T extends ModInner, C extends ModChange> =
  C extends { name: string } ? Omit<T, 'name'> & Pick<C, 'name'> :
  C extends { author: string } ? Omit<T, 'author'> & Pick<C, 'author'> :
  C extends { description: VersionMap<string> } ? Omit<T, 'description'> & Pick<C, 'description'> :
  C extends { dependencies: VersionMap<ModDependency[]> } ? Omit<T, 'dependencies'> & Pick<C, 'dependencies'> :
  C extends { supportedVersion: Version[] } ? Omit<T, 'supportedVersion'> & Pick<C, 'supportedVersion'> :
  C extends { loadOrder: VersionMap<ModOrder[]> } ? Omit<T, 'loadOrder'> & Pick<C, 'loadOrder'> :
  C extends { incompatibleWith: VersionMap<PackageId[]> } ? Omit<T, 'incompatibleWith'> & Pick<C, 'incompatibleWith'> :
  C extends { supportLanguages: VersionMap<string[]> } ? Omit<T, 'supportLanguages'> & Pick<C, 'supportLanguages'> :
  C extends { enabled: boolean } ? Omit<T, 'enabled'> & Pick<C, 'enabled'> :
  C extends { displayName: string } ? Omit<T, 'displayName'> & Pick<C, 'displayName'> :
  T;

// 工具函数
export function applyModChanges(mod: ModInner, changes: ModChange[]): ModInner {
  return changes.reduce<ModInner>((acc, change) => ({
    ...acc,
    ...change
  }), { ...mod });
}

export interface VersionMapInterface<T> {
  insert(version: string, value: T): void;
  get(version: string): T | undefined;
  getWithVersion(version: string): [string, T] | undefined;
  toObject(): Record<string, T>;
  keys(): IterableIterator<string>;
  values(): IterableIterator<T>;
  entries(): IterableIterator<[string, T]>;
  clear(): void;
  delete(version: string): boolean;
  has(version: string): boolean;
}

export class VersionMap<T> implements VersionMapInterface<T> {
  private map: Map<string, T>;

  constructor() {
    this.map = new Map<string, T>();
  }

  // 插入数据
  insert(version: string, value: T): void {
    this.map.set(version, value);
  }

  // 获取指定版本的数据,如果没有完全匹配则返回通配符匹配
  get(version: string): T | undefined {
    // 先尝试精确匹配
    if (this.map.has(version)) {
      return this.map.get(version);
    }

    // 检查是否有通配符匹配
    if (this.map.has('*')) {
      return this.map.get('*');
    }

    // 尝试模式匹配
    for (const [key, value] of this.map.entries()) {
      if (this.matchVersion(version, key)) {
        return value;
      }
    }

    return undefined;
  }

  getWithVersion(version: string): [string, T] | undefined {
    if (this.map.has(version)) {
      return [version, this.map.get(version)!];
    }

    // 检查是否有通配符匹配
    if (this.map.has('*')) {
      return ['*', this.map.get('*')!];
    }

    // 尝试模式匹配
    for (const [key, value] of this.map.entries()) {
      if (this.matchVersion(version, key)) {
        return [key, value];
      }
    }

    return undefined;
  }

  // 从普通Map创建VersionMap
  static fromMap<T>(map: Record<string, T>): VersionMap<T> {
    const versionMap = new VersionMap<T>();
    for (const [version, value] of Object.entries(map)) {
      versionMap.insert(version, value);
    }
    return versionMap;
  }

  // 版本号匹配逻辑
  private matchVersion(target: string, pattern: string): boolean {
    // 通配符匹配
    if (pattern === '*') return true;

    // 精确匹配
    if (target === pattern) return true;

    // 主版本号匹配 (例如 1.4 匹配 1.4.x)
    const targetParts = target.split('.');
    const patternParts = pattern.split('.');

    // 如果模式比目标短,则进行前缀匹配
    if (patternParts.length < targetParts.length) {
      for (let i = 0; i < patternParts.length; i++) {
        if (patternParts[i] !== targetParts[i]) return false;
      }
      return true;
    }

    return false;
  }

  // 转换为普通对象
  toObject(): Record<string, T> {
    return Object.fromEntries(this.map);
  }

  // 获取所有版本
  keys(): IterableIterator<string> {
    return this.map.keys();
  }

  // 获取所有值
  values(): IterableIterator<T> {
    return this.map.values();
  }

  // 获取所有条目
  entries(): IterableIterator<[string, T]> {
    return this.map.entries();
  }

  // 清空Map
  clear(): void {
    this.map.clear();
  }

  // 删除指定版本
  delete(version: string): boolean {
    return this.map.delete(version);
  }

  // 检查版本是否存在
  has(version: string): boolean {
    return this.map.has(version);
  }
}


export interface SteamDatabase {
  appid?: boolean;
  url?: string;
  packageId?: string;
  name?: string;
  authors?: string[] | null | string;
  gameVersions?: string[] | string;
  packageid?: string;
  steamName?: string;
  dependencies?: { [key: string]: string[] };
  unpublished?: boolean;
  external_time_created?: number;
  external_time_updated?: number;
  blacklist?: SteamDBBlacklist;
}

export interface SteamDBBlacklist {
  value: boolean;
  comment: string;
}

export interface ScanResult {
  info: Record<string, ScanInfoType[]>;
}

export type ScanInfoType = {
  Warning: ScanWarning;
} | {
  Error: ScanError;
}

export type ScanWarning = 'VersionMismatch'

export type ScanError = {
  DuplicatePackageId: Id;
} | {
  MissingDependency: ModDependency;
}

export interface TranslateResponse {
  code: number;
  message: string | null;
  data: string;
  source: string;
  target: string;
}

export interface SaveMetaData {
  save_time: string; // rfc3339 
  mods_count: string;
  mods_groups_count: string;
}


export interface CustomCalcResult {
  overall_score: number;
  theshold: number;
  name: CustomCalcNameResult;
  package_id: CustomCalcPackageIdResult;
  additional: Record<string, string>;
}

export interface CustomCalcNameResult {
  score: number;
  details: Record<string, string>;
}

export interface CustomCalcPackageIdResult {
  score: number;
  details: Record<string, string>;
}