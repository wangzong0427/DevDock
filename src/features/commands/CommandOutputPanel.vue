<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import type { CommandRunOutput } from "../../types";

const props = defineProps<{
  runOutput: CommandRunOutput | null;
  runningCommandName: string | null;
}>();

const stdoutRef = ref<HTMLElement | null>(null);
const stderrRef = ref<HTMLElement | null>(null);

function exitLabel(output: CommandRunOutput) {
  if (output.status === "running") return "执行中";
  return output.exitCode === null || output.exitCode === undefined
    ? "未返回退出码"
    : `退出码 ${output.exitCode}`;
}

function statusLabel(output: CommandRunOutput) {
  if (output.status === "running") return "执行中";
  return output.status === "success" ? "执行成功" : "执行失败";
}

function statusType(output: CommandRunOutput) {
  if (output.status === "running") return "warning";
  return output.status === "success" ? "success" : "danger";
}

function scrollToBottom(element: HTMLElement | null) {
  if (!element) return;
  element.scrollTop = element.scrollHeight;
}

watch(
  () => [props.runOutput?.stdout, props.runOutput?.stderr],
  async () => {
    await nextTick();
    scrollToBottom(stdoutRef.value);
    scrollToBottom(stderrRef.value);
  },
);
</script>

<template>
  <el-card class="command-output-panel" shadow="never">
    <template #header>
      <div class="card-header">
        <h3>运行输出</h3>
        <el-tag v-if="runOutput" :type="statusType(runOutput)" effect="light">
          {{ statusLabel(runOutput) }}
        </el-tag>
      </div>
    </template>

    <section v-if="runOutput" class="command-run-result" aria-label="命令运行输出">
      <div class="run-result-header">
        <div>
          <strong>{{ runOutput.commandName }}</strong>
          <span>{{ exitLabel(runOutput) }}</span>
        </div>
      </div>

      <div class="run-output-stack">
        <div class="run-output">
          <span>标准输出</span>
          <pre ref="stdoutRef">{{ runOutput.stdout || (runningCommandName ? "等待输出..." : "无输出") }}</pre>
        </div>
        <div class="run-output">
          <span>错误输出</span>
          <pre ref="stderrRef">{{ runOutput.stderr || "无输出" }}</pre>
        </div>
      </div>
    </section>

    <el-empty v-else class="command-output-empty" description="暂无输出">
      <span>执行命令后会在这里显示实时日志。</span>
    </el-empty>
  </el-card>
</template>
