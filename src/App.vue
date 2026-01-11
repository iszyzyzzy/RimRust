<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { darkTheme, zhCN, dateZhCN } from 'naive-ui'
import { getBaseList, listenSyncMany, listenStartLoading, listenEndLoading, getTranslationUnconfirmed, getTranslationPack, loadSaveMetaData } from './api/tauriFunc';
import { invoke } from '@tauri-apps/api/core'

import { DateTime } from "luxon";

import Header from './components/header/Main.vue';
import LoadingModal from './components/Loading.vue';
import Footer from './components/footer/Footer.vue';
import MainContext from './components/main/MainContext.vue';
import { SaveMetaData } from './api/types';
import { useBaseListStore, useInitdStore } from './components/utils/store';

const baseListStore = useBaseListStore()
const initd = useInitdStore()

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))

const init = async (useAutoSave: boolean) => {

  console.log('command started_mission, useAutoSave:', useAutoSave)
  console.time('init_mission')

  await invoke('init_mission', {loadFromAutosave: useAutoSave})

  initd.$patch({ inited: true })

  console.log('command getBaseList')
  // 只在这里拿一次全量数据，剩下的全部交给 sync many
  getBaseList().then((res) => {
    console.log('BaseList first sync',res)
    baseListStore.$patch(res)

    listenSyncMany((msg) => {
      console.log("received sync many", msg)
      baseListStore.handleSyncManyPayload(msg.payload)
    })

  console.timeEnd('init_mission')
  })
}

const selectSaveModal = ref(false)
const saveMetaData = ref<[SaveMetaData | null, SaveMetaData | null]>([null, null])

onMounted(async () => {
  console.log('App mounted!')
  console.time('App mounted')
  listenStartLoading((msg) => {
    loading.value = true
    loadingContext.value = msg.payload
  })
  listenEndLoading(() => {
    loading.value = false
  })
  await sleep(100)
  saveMetaData.value = await loadSaveMetaData()
  console.log('saveMetaData', saveMetaData.value)
  let useAutoSave = false
  if (saveMetaData.value[1] && !saveMetaData.value[0]) {
    useAutoSave = true
    init(useAutoSave)
  } else if (saveMetaData.value[1] && saveMetaData.value[0]) {
    if (DateTime.fromISO(saveMetaData.value[1].save_time) > (DateTime.fromISO(saveMetaData.value[0].save_time))) {
      selectSaveModal.value = true
    } else {
      init(useAutoSave)
    }
  } else {
    init(useAutoSave)
  }
  console.timeEnd('App mounted')
})

const handleSelectSave = (useAutoSave: boolean) => {
  selectSaveModal.value = false
  init(useAutoSave)
}

const loading = ref(false)
const loadingContext = ref('')

import hljs from 'highlight.js/lib/core'

</script>

<template>
  <n-config-provider :theme="darkTheme" :locale="zhCN" :date-locale="dateZhCN" :hljs="hljs">
    <n-message-provider :max="3">
      <n-modal-provider>
        <div id="teleported">
          <!-- 这个div是用来给我自己写的几个Teleport做终点的，不然传送到body上吃不到config -->
        </div>
        <n-layout>
          <n-layout-header style="height: 34px;">
            <Header/>
          </n-layout-header>
          <n-layout-content content-style="padding: 0 6px; height: calc(100vh - 68px);">
            <MainContext />
          </n-layout-content>
          <n-layout-footer style="height: 34px;">
            <Footer />
          </n-layout-footer>
        </n-layout>
        <LoadingModal :show="loading" :context="loadingContext" />
        <n-modal 
          :show="selectSaveModal"
          title="发现了更新的自动存档，可能是上次应用没有正常关闭？"
          preset="card"
          style="width: 600px;"
        >
        <n-flex vertical>
        <n-card class="save-card" @click="handleSelectSave(true)">
          <n-descriptions label-placement="top" title="自动存档"
          >
            <n-descriptions-item label="保存时间">
              {{ DateTime.fromISO(saveMetaData[1]!.save_time).setLocale('zh-cn').toRelative() }}
              ({{ DateTime.fromISO(saveMetaData[1]!.save_time).setLocale('zh-cn').toLocaleString(DateTime.DATETIME_MED_WITH_SECONDS) }})
            </n-descriptions-item>
            <n-descriptions-item label="mod数量">{{ saveMetaData[1]?.mods_count }}</n-descriptions-item>
          </n-descriptions>
        </n-card>
        <n-card class="save-card" @click="handleSelectSave(false)">
          <n-descriptions label-placement="top" title="手动存档"
          >
            <n-descriptions-item label="保存时间">
              {{ DateTime.fromISO(saveMetaData[0]!.save_time).setLocale('zh-cn').toRelative() }}
              ({{ DateTime.fromISO(saveMetaData[0]!.save_time).setLocale('zh-cn').toLocaleString(DateTime.DATETIME_MED_WITH_SECONDS) }})
            </n-descriptions-item>
            <n-descriptions-item label="mod数量">{{ saveMetaData[0]?.mods_count }}</n-descriptions-item>
          </n-descriptions>
        </n-card>
      </n-flex>
        </n-modal>
      </n-modal-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<style scoped>
.save-card {
    transition: background-color 0.1s ease;
}

.save-card:hover {
    background-color: #394753;
}
</style>
<!-- <style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}

button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }

  button:active {
    background-color: #0f0f0f69;
  }
}
</style> -->