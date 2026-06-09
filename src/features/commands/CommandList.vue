<script setup lang="ts">
import { VideoPlay } from "@element-plus/icons-vue";
import type { CommandRunResult, EntryType, RegisteredCommand } from "../../types";

defineProps<{
  commands: RegisteredCommand[];
  runningCommandName: string | null;
  lastRunResult: CommandRunResult | null;
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

function exitLabel(result: CommandRunResult) {
  return result.exitCode === null || result.exitCode === undefined
    ? "未返回退出码"
    : `退出码 ${result.exitCode}`;
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

    <section v-if="lastRunResult" class="command-run-result" aria-label="最近一次执行结果">
      <div class="run-result-header">
        <div>
          <strong>{{ lastRunResult.commandName }}</strong>
          <span>{{ exitLabel(lastRunResult) }}</span>
        </div>
        <el-tag :type="lastRunResult.exitCode === 0 ? 'success' : 'danger'" effect="light">
          {{ lastRunResult.exitCode === 0 ? "执行成功" : "执行失败" }}
        </el-tag>
      </div>
      <div class="run-output-grid">
        <div class="run-output">
          <span>标准输出</span>
          <pre>{{ lastRunResult.stdout || "无输出" }}</pre>
        </div>
        <div class="run-output">
          <span>错误输出</span>
          <pre>{{ lastRunResult.stderr || "无输出" }}</pre>
        </div>
      </div>
    </section>
  </el-card>
</template>
