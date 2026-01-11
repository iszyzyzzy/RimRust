<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useAppConfigStore, AppConfig } from "../utils/store/index.ts";
import { invoke } from "@tauri-apps/api/core";
import { InfoOutlined } from "@vicons/material";

import { useMessage } from "naive-ui";
const message = useMessage();
const appConfigStore = useAppConfigStore();


// [string, string, string] 0: label, 1: description, 2: type
const pathData: Record<string, [string, string, string]> = {
  game_path: ["游戏路径", "RimSort设置里的Game location", "string"],
  game_config_path: ["游戏配置路径", "RimSort设置里的Config location", "string"],
  steam_mods_path: ["创意工坊mod路径", "RimSort设置里的Steam mods location", "string"],
  community_rules_update_path: ["社区排序规则更新地址", "默认地址就是来自rimsort的地址", "string"],
  steam_db_update_path: ["创意工坊数据库更新地址", "默认地址就是来自rimsort的地址", "string"],
  proxy: ["代理", "更新上面两个文件以及翻译请求时的代理, 使用reqwest格式(protocol://address:port)", "string"],
}
const pathConfig = computed(() => {
  let res = [];
  for (let key in appConfigStore.$state) {
    if (pathData[key]) {
      res.push({
        label: pathData[key][0],
        description: pathData[key][1],
        key: key,
        value: appConfigStore.$state[key as keyof typeof appConfigStore.$state],
        type: pathData[key][2],
      });
    }
  }
  return res;
});

const searchData: Record<string, [string, string, string]> = {
  use_advance_search: ["使用高级搜索", "高级搜索是基于tantivy的全文搜索，目前还未完工", "boolean"],
}

const searchConfig = computed(() => {
  let res = [];
  for (let key in appConfigStore.$state) {
    if (searchData[key]) {
      res.push({
        label: searchData[key][0],
        description: searchData[key][1],
        key: key,
        value: appConfigStore.$state[key as keyof typeof appConfigStore.$state],
        type: searchData[key][2],
      });
    }
  }
  return res;
});

const translateData: Record<string, [string, string, string]> = {
  prefer_language: ["首选语言", "如果你能看懂这句话，我想你应该是不用动这个的", "string"],
}

const translateConfig = computed(() => {
  let res = [];
  for (let key in appConfigStore.$state) {
    if (translateData[key]) {
      res.push({
        label: translateData[key][0],
        description: translateData[key][1],
        key: key,
        value: appConfigStore.$state[key as keyof typeof appConfigStore.$state],
        type: translateData[key][2],
      });
    }
  }
  return res;
});

const updateConfig = (key: string, value: any) => {
  appConfigStore.$patch((state) => {
    (state as any)[key] = value;
  });
};
const showAppConfigModal = defineModel<boolean>('show');


onMounted(() => {
  invoke<AppConfig>("config_get").then((res) => {
    console.log("config_get", res);
    appConfigStore.$patch(res);
    if (
      res.game_config_path === "" ||
      res.steam_mods_path === "" ||
      res.game_path === ""
    ) {
      showAppConfigModal.value = true;
      message.error("请先设置游戏路径、游戏配置路径和Steam工坊路径");
    }
  });
});
const handleSave = () => {
  invoke("config_set", { appConfig: appConfigStore.$state }).then(() => {
    message.success("保存成功");
  });
  showAppConfigModal.value = false;
};

const rawConfig = ref(
  JSON.stringify(appConfigStore.$state, null, 2)
)
const handleRawConfigChange = (v: string) => {
  try {
    const obj: AppConfig = JSON.parse(v);
    appConfigStore.$patch(obj);
    rawConfig.value = JSON.stringify(obj, null, 2);
  } catch (e) {
    message.error("JSON格式错误");
  }
}
</script>

<template>
  <n-modal :show="showAppConfigModal">
    <n-card style="width: 800px" title="应用设置" :bordered="false" size="huge" role="dialog" aria-modal="true">
        <n-tabs size="large" animated>
          <n-tab-pane name="path" tab="路径">
            <n-scrollbar style="height: 500px">
              <template v-for="item in pathConfig" :key="item.key">
                <n-flex vertical>
                  <n-flex style="padding: 10px" align="center">
                    <div>{{ item.label }}</div>
                    <n-tooltip trigger="hover" v-if="!(item.description === '')">
                      <template #trigger>
                        <n-icon>
                          <InfoOutlined />
                        </n-icon>
                      </template>
                      {{ item.description }}
                    </n-tooltip>
                    <n-input :value="item.value" type="text" @update:value="(v: string) => updateConfig(item.key, v)"
                      v-if="item.type === 'string'" />
                    <template v-else-if="item.type === 'number'">
                      <n-input-number :value="item.value" @update:value="(v: number) => updateConfig(item.key, v)" />
                    </template>
                    <template v-else-if="item.type === 'boolean'">
                      <n-switch :value="item.value" @update:value="(v: boolean) => updateConfig(item.key, v)" />
                    </template>
                  </n-flex>
                </n-flex>
              </template>
            </n-scrollbar>
          </n-tab-pane>
          <n-tab-pane name="search" tab="搜索">
            <n-scrollbar style="height: 500px">
            <template v-for="item in searchConfig" :key="item.key">
              <n-flex vertical>
                <n-flex style="padding: 10px" align="center">
                  <div>{{ item.label }}</div>
                  <n-tooltip trigger="hover" v-if="!(item.description === '')">
                    <template #trigger>
                      <n-icon>
                        <InfoOutlined />
                      </n-icon>
                    </template>
                    {{ item.description }}
                  </n-tooltip>
                  <n-input :value="item.value" type="text" @update:value="(v: string) => updateConfig(item.key, v)"
                    v-if="item.type === 'string'" />
                  <template v-else-if="item.type === 'number'">
                    <n-input-number :value="item.value" @update:value="(v: number) => updateConfig(item.key, v)" />
                  </template>
                  <template v-else-if="item.type === 'boolean'">
                    <n-switch :value="item.value" @update:value="(v: boolean) => updateConfig(item.key, v)" />
                  </template>
                </n-flex>
              </n-flex>
            </template>
          </n-scrollbar>
          </n-tab-pane>
          <n-tab-pane name="translate" tab="翻译">
            <n-scrollbar style="height: 500px">
            <template v-for="item in translateConfig" :key="item.key">
              <n-flex vertical>
                <n-flex style="padding: 10px" align="center">
                  <div>{{ item.label }}</div>
                  <n-tooltip trigger="hover" v-if="!(item.description === '')">
                    <template #trigger>
                      <n-icon>
                        <InfoOutlined />
                      </n-icon>
                    </template>
                    {{ item.description }}
                  </n-tooltip>
                  <n-input :value="item.value" type="text" @update:value="(v: string) => updateConfig(item.key, v)"
                    v-if="item.type === 'string'" />
                  <template v-else-if="item.type === 'number'">
                    <n-input-number :value="item.value" @update:value="(v: number) => updateConfig(item.key, v)" />
                  </template>
                  <template v-else-if="item.type === 'boolean'">
                    <n-switch :value="item.value" @update:value="(v: boolean) => updateConfig(item.key, v)" />
                  </template>
                </n-flex>
              </n-flex>
            </template>
          </n-scrollbar>
          </n-tab-pane>
          <n-tab-pane name="advance" tab="高级">
            <n-input type="textarea" :autosize="{
              minRows: 3,
            }" :value="rawConfig" @update:value="handleRawConfigChange" />
          </n-tab-pane>
        </n-tabs>
      <template #footer>
        <n-flex justify="end">
          <n-button @click="showAppConfigModal = false">取消</n-button>
          <n-button type="primary" @click="handleSave">确定</n-button>
        </n-flex>
      </template>
    </n-card>
  </n-modal>
</template>
