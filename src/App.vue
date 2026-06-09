<script setup lang="ts">
import { ElMessage } from "element-plus";
import { computed, ref } from "vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import SidebarNav from "./components/SidebarNav.vue";
import AdbView from "./features/adb/AdbView.vue";
import CommandsView from "./features/commands/CommandsView.vue";
import type { ActiveModule, PathStatus, PlatformInfo, RegisteredCommand } from "./types";

const activeModule = ref<ActiveModule>("commands");
const platformInfo = ref<PlatformInfo>({ name: "macOS" });
const pathStatus = ref<PathStatus>({
  state: "missing",
  binDir: "~/.local/bin",
  message: "命令目录未加入 PATH。",
  suggestedCommand: "export PATH=\"$HOME/.local/bin:$PATH\"",
});

const scriptPath = ref("/Users/me/dev/scripts/deploy-preview.sh");
const commandName = ref("deploy-preview");
const isRegistering = ref(false);
const isRefreshing = ref(false);
const commandToDelete = ref<RegisteredCommand | null>(null);

const commands = ref<RegisteredCommand[]>([
  {
    name: "sync-env",
    scriptPath: "/Users/me/dev/scripts/sync-env.sh",
    entryPath: "~/.local/bin/sync-env",
    entryType: "wrapper",
    createdAt: "2024-05-14 10:32",
  },
  {
    name: "clean-cache",
    scriptPath: "/Users/me/dev/scripts/clean-cache.sh",
    entryPath: "~/.local/bin/clean-cache",
    entryType: "symlink",
    createdAt: "2024-05-14 10:28",
  },
]);

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
  return `~/.local/bin/${name}`;
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

function chooseMockScript() {
  scriptPath.value = "/Users/me/dev/scripts/deploy-preview.sh";
  if (!commandName.value) {
    commandName.value = "deploy-preview";
  }
}

function refreshCommands() {
  isRefreshing.value = true;
  window.setTimeout(() => {
    isRefreshing.value = false;
    pushToast("info", "命令列表已刷新。");
  }, 500);
}

function registerCommand() {
  if (!canRegister.value) return;

  const name = commandName.value;
  const createdAt = new Date().toLocaleString([], {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });

  isRegistering.value = true;
  window.setTimeout(() => {
    commands.value = [
      {
        name,
        scriptPath: scriptPath.value,
        entryPath: entryPreview.value,
        entryType: platformInfo.value.name === "Windows" ? "cmd-shim" : "wrapper",
        createdAt,
      },
      ...commands.value,
    ];
    scriptPath.value = "";
    commandName.value = "";
    isRegistering.value = false;
    pushToast("success", `已注册 ${name}。`);
  }, 650);
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
  pushToast("info", `后端接入后会打开 ${command.entryPath}。`);
}

function deleteCommand() {
  if (!commandToDelete.value) return;

  const deletedName = commandToDelete.value.name;
  commands.value = commands.value.filter((command) => command.name !== deletedName);
  commandToDelete.value = null;
  pushToast("success", `已删除 ${deletedName}。`);
}
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
          @browse="chooseMockScript"
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
