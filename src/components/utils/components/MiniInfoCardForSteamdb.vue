<template>
    <MiniInfoCard :data="innerData" show-border/>
</template>
<script setup lang="ts">
import { computed } from 'vue';
import { ModInner, SteamDatabase, VersionMap } from '../../../api/types';
import MiniInfoCard from './MiniInfoCard.vue';

const { data } = defineProps<{
    data: SteamDatabase
}>()

const innerData = computed<ModInner>(() => {
    let t_author = data.authors
                        if(typeof t_author === 'string') {
                            t_author = [t_author]
                        }
                        t_author = t_author?.join(", ")
    return {
        id: "",
        enabled: false,
        visible: true,
        packageId: data.packageId,
        name: data.name,
        author: t_author,
        displayName: data.name,
        description: new VersionMap(),
        dependencies: new VersionMap(),
        supportedVersion: [],
        path: "",
        loadOrder: new VersionMap(),
        incompatibleWith: new VersionMap(),
        supportLanguages: new VersionMap(),
        displayOrder: 0,
    } as ModInner
})
</script>