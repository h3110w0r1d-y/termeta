<script setup lang="ts">
import { NCollapse, NCollapseItem } from "naive-ui";
defineProps<{ title: string; defaultOpen?: boolean }>();
</script>
<template>
  <NCollapse :default-expanded-names="defaultOpen ? ['content'] : []">
    <NCollapseItem name="content" :title="title"><slot /></NCollapseItem>
  </NCollapse>
</template>
