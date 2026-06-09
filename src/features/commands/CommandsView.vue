<script setup lang="ts">
import { Refresh } from "@element-plus/icons-vue";
import type { PathStatus, RegisteredCommand } from "../../types";
import CommandList from "./CommandList.vue";
import PathStatusPanel from "./PathStatusPanel.vue";
import RegisterCommandPanel from "./RegisterCommandPanel.vue";

defineProps<{
  scriptPath: string;
  commandName: string;
  commandNameError: string;
  entryPreview: string;
  canRegister: boolean;
  isRegistering: boolean;
  isRefreshing: boolean;
  pathStatus: PathStatus;
  pathTone: string;
  commands: RegisteredCommand[];
}>();

const emit = defineEmits<{
  "update:scriptPath": [value: string];
  "update:commandName": [value: string];
  browse: [];
  register: [];
  refresh: [];
  copyPath: [];
  revealCommand: [command: RegisteredCommand];
  requestDelete: [command: RegisteredCommand];
}>();
</script>

<template>
  <section class="commands-view" aria-labelledby="commands-title">
    <header class="page-header">
      <div>
        <h2 id="commands-title">命令</h2>
        <p>把本地脚本注册成可以在终端里直接调用的快捷命令。</p>
      </div>
      <el-button type="primary" plain :icon="Refresh" :loading="isRefreshing" @click="emit('refresh')">
        刷新
      </el-button>
    </header>

    <RegisterCommandPanel
      :script-path="scriptPath"
      :command-name="commandName"
      :command-name-error="commandNameError"
      :entry-preview="entryPreview"
      :can-register="canRegister"
      :is-registering="isRegistering"
      @update:script-path="emit('update:scriptPath', $event)"
      @update:command-name="emit('update:commandName', $event)"
      @browse="emit('browse')"
      @register="emit('register')"
    />

    <PathStatusPanel :path-status="pathStatus" :path-tone="pathTone" @copy="emit('copyPath')" />

    <CommandList
      :commands="commands"
      @reveal="emit('revealCommand', $event)"
      @delete="emit('requestDelete', $event)"
    />
  </section>
</template>
