<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { ElMessage } from "element-plus";
import { computed, onMounted, ref } from "vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import SidebarNav from "./components/SidebarNav.vue";
import AdbView from "./features/adb/AdbView.vue";
import CommandsView from "./features/commands/CommandsView.vue";
import type { ActiveModule, PathStatus, PlatformInfo, RegisteredCommand } from "./types";

const activeModule = ref<ActiveModule>("commands");
const platformInfo = ref<PlatformInfo>({ name: "macOS" });
const pathStatus = ref<PathStatus>({
  state: "checking",
  binDir: "",
  message: "正在读取系统 PATH...",
  suggestedCommand: "export PATH=\"$HOME/.local/bin:$PATH\"",
});

const scriptPath = ref("");
const commandName = ref("");
const isRegistering = ref(false);
const isRefreshing = ref(false);
const commandToDelete = ref<RegisteredCommand | null>(null);
const commands = ref<RegisteredCommand[]>([]);

const commandNamePattern = /^[A-Za-z0-9_.-]+$/;

const commandNameError = computed(() => {
  if (!commandName.value) return "";
  if (!commandNamePattern.test(commandName.value)) {
    return "只能使用字母、数字、点、短横线或下划线。";
  }
  if (commands.value.some((command) => command.name === commandName.value)) {
    return "该命令名已存在。";
  }
  return "";
});

const canRegister = computed(() => {
  return Boolean(scriptPath.value && commandName.value && !commandNameError.value);
});

const entryPreview = computed(() => {
  const name = commandName.value || "my-command";
  if (platformInfo.value.name === "Windows") {
    return `%LOCALAPPDATA%\\devdock\\bin\\${name}.cmd`;
  }
  const binDir = pathStatus.value.binDir || "~/.local/bin";
  return `${binDir}/${name}`;
});

const pathTone = computed(() => {
  if (pathStatus.value.state === "ok") return "ok";
  if (pathStatus.value.state === "missing") return "warning";
  if (pathStatus.value.state === "error") return "danger";
  return "muted";
});

const pathStatusLabel = computed(() => {
  if (pathStatus.value.state === "ok") return "PATH 正常";
  if (pathStatus.value.state === "missing") return "PATH 未配置";
  if (pathStatus.value.state === "error") return "PATH 异常";
  return "检查中";
});

function pushToast(tone: "success" | "error" | "info", text: string) {
  ElMessage({
    message: text,
    type: tone,
    duration: 2600,
    showClose: true,
  });
}

async function refreshPathStatus() {
  pathStatus.value = {
    ...pathStatus.value,
    state: "checking",
    message: "正在读取系统 PATH...",
  };

  try {
    pathStatus.value = await invoke<PathStatus>("get_path_status");
  } catch (error) {
    pathStatus.value = {
      state: "error",
      binDir: pathStatus.value.binDir || "~/.local/bin",
      message: `读取 PATH 失败：${String(error)}`,
      suggestedCommand: pathStatus.value.suggestedCommand || "export PATH=\"$HOME/.local/bin:$PATH\"",
      paths: [],
    };
    pushToast("error", "读取 PATH 失败。");
  }
}

async function refreshRegisteredCommands() {
  try {
    commands.value = await invoke<RegisteredCommand[]>("list_registered_commands");
  } catch (error) {
    commands.value = [];
    pushToast("error", `读取命令列表失败：${String(error)}`);
  }
}

async function refreshCommands() {
  isRefreshing.value = true;
  await Promise.all([
    refreshPathStatus(),
    refreshRegisteredCommands(),
    new Promise((resolve) => {
      window.setTimeout(resolve, 500);
    }),
  ]);
  isRefreshing.value = false;
  pushToast("info", "命令列表和 PATH 状态已刷新。");
}

async function registerCommand() {
  if (!canRegister.value) return;

  const name = commandName.value.trim();
  isRegistering.value = true;
  try {
    await invoke<RegisteredCommand>("register_command", {
      scriptPath: scriptPath.value.trim(),
      commandName: name,
    });
    await refreshRegisteredCommands();
    scriptPath.value = "";
    commandName.value = "";
    pushToast("success", `已注册 ${name}。`);
  } catch (error) {
    pushToast("error", `注册失败：${String(error)}`);
  } finally {
    isRegistering.value = false;
  }
}

async function copyPathCommand() {
  const command = pathStatus.value.suggestedCommand || pathStatus.value.binDir;
  try {
    await navigator.clipboard.writeText(command);
    pushToast("success", "PATH 修复命令已复制。");
  } catch {
    pushToast("error", "复制失败，请手动选择命令。");
  }
}

function revealCommand(command: RegisteredCommand) {
  pushToast("info", `命令入口：${command.entryPath}`);
}

async function deleteCommand() {
  if (!commandToDelete.value) return;

  const deletedName = commandToDelete.value.name;
  try {
    await invoke("delete_registered_command", { commandName: deletedName });
    await refreshRegisteredCommands();
    commandToDelete.value = null;
    pushToast("success", `已删除 ${deletedName}。`);
  } catch (error) {
    pushToast("error", `删除失败：${String(error)}`);
  }
}

onMounted(() => {
  void refreshPathStatus();
  void refreshRegisteredCommands();
});
</script>

<template>
  <el-container class="app-shell">
    <SidebarNav
      v-model:active-module="activeModule"
      :platform-info="platformInfo"
      :path-tone="pathTone"
      :path-status-label="pathStatusLabel"
    />

    <el-container class="content-shell">
      <el-main class="workspace">
        <CommandsView
          v-if="activeModule === 'commands'"
          v-model:script-path="scriptPath"
          v-model:command-name="commandName"
          :command-name-error="commandNameError"
          :entry-preview="entryPreview"
          :can-register="canRegister"
          :is-registering="isRegistering"
          :is-refreshing="isRefreshing"
          :path-status="pathStatus"
          :path-tone="pathTone"
          :commands="commands"
          @register="registerCommand"
          @refresh="refreshCommands"
          @copy-path="copyPathCommand"
          @reveal-command="revealCommand"
          @request-delete="commandToDelete = $event"
        />
        <AdbView v-else />
      </el-main>
    </el-container>

    <ConfirmDialog :command="commandToDelete" @cancel="commandToDelete = null" @confirm="deleteCommand" />
  </el-container>
</template>
