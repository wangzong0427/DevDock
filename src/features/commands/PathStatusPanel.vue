<script setup lang="ts">
import { CopyDocument } from "@element-plus/icons-vue";
import { computed } from "vue";
import type { PathStatus } from "../../types";

const props = defineProps<{
  pathStatus: PathStatus;
  pathTone: string;
}>();

const emit = defineEmits<{
  copy: [];
}>();

const alertType = computed(() => {
  if (props.pathStatus.state === "ok") return "success";
  if (props.pathStatus.state === "error") return "error";
  if (props.pathStatus.state === "missing") return "warning";
  return "info";
});
</script>

<template>
  <el-card class="path-card" :class="pathTone" shadow="never">
    <el-alert title="PATH 状态" :type="alertType" :description="pathStatus.message" show-icon :closable="false" />
    <div class="path-tools">
      <el-input :model-value="pathStatus.binDir" readonly />
      <el-button
        v-if="pathStatus.suggestedCommand"
        type="primary"
        plain
        :icon="CopyDocument"
        @click="emit('copy')"
      >
        复制修复命令
      </el-button>
    </div>
  </el-card>
</template>
