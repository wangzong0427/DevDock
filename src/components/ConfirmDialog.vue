<script setup lang="ts">
import type { RegisteredCommand } from "../types";

defineProps<{
  command: RegisteredCommand | null;
}>();

const emit = defineEmits<{
  cancel: [];
  confirm: [];
}>();
</script>

<template>
  <el-dialog
    :model-value="Boolean(command)"
    title="删除命令？"
    width="min(420px, calc(100vw - 32px))"
    align-center
    @close="emit('cancel')"
  >
    <p v-if="command" class="confirm-copy">
      将删除
      <strong>{{ command.name }}</strong>
      对应的生成入口，原始脚本文件不会被删除。
    </p>
    <template #footer>
      <el-button @click="emit('cancel')">取消</el-button>
      <el-button type="danger" @click="emit('confirm')">删除</el-button>
    </template>
  </el-dialog>
</template>
