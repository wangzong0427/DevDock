<script setup lang="ts">
import { VideoPlay } from "@element-plus/icons-vue";
import type { EntryType, RegisteredCommand } from "../../types";

defineProps<{
  commands: RegisteredCommand[];
  runningCommandName: string | null;
}>();

const emit = defineEmits<{
  run: [command: RegisteredCommand];
  reveal: [command: RegisteredCommand];
  delete: [command: RegisteredCommand];
}>();

function entryTypeLabel(type: EntryType) {
  if (type === "symlink") return "符号链接";
  if (type === "wrapper") return "包装脚本";
  if (type === "cmd-shim") return "CMD 入口";
  return "PS1 入口";
}

function entryTypeTag(type: EntryType) {
  if (type === "symlink") return "primary";
  if (type === "wrapper") return "success";
  return "info";
}
</script>

<template>
  <el-card class="command-section" shadow="never">
    <template #header>
      <div class="card-header">
        <h3 id="list-title">已注册命令</h3>
        <el-tag type="info" round>{{ commands.length }}</el-tag>
      </div>
    </template>

    <div v-if="commands.length" class="command-list" aria-labelledby="list-title">
      <article v-for="command in commands" :key="command.name" class="command-row">
        <div class="command-name">
          <strong>{{ command.name }}</strong>
          <span>{{ command.entryPath }}</span>
        </div>
        <span class="command-script" :title="command.scriptPath">{{ command.scriptPath }}</span>
        <el-tag class="command-entry" :type="entryTypeTag(command.entryType)" effect="light">
          {{ entryTypeLabel(command.entryType) }}
        </el-tag>
        <span class="command-created">{{ command.createdAt }}</span>
        <div class="command-actions">
          <el-button
            type="success"
            link
            :icon="VideoPlay"
            :loading="runningCommandName === command.name"
            :disabled="Boolean(runningCommandName && runningCommandName !== command.name)"
            @click="emit('run', command)"
          >
            执行
          </el-button>
          <el-button type="primary" link @click="emit('reveal', command)">定位</el-button>
          <el-button type="danger" link :disabled="runningCommandName === command.name" @click="emit('delete', command)">
            删除
          </el-button>
        </div>
      </article>
    </div>
    <el-empty v-else class="command-empty" description="还没有注册命令">
      <span>选择一个脚本，创建第一个终端快捷命令。</span>
    </el-empty>
  </el-card>
</template>
