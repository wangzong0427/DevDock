<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { ElMessage } from "element-plus";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import SidebarNav from "./components/SidebarNav.vue";
import AdbView from "./features/adb/AdbView.vue";
import CommandsView from "./features/commands/CommandsView.vue";
import UpdaterView from "./features/updater/UpdaterView.vue";
import type {
  ActiveModule,
  CommandOutputChunk,
  CommandRunFailed,
  CommandRunFinished,
  CommandRunOutput,
  CommandRunResult,
  CommandRunStarted,
  PathStatus,
  PlatformInfo,
  RegisteredCommand,
} from "./types";

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
const runningCommandName = ref<string | null>(null);
const runOutput = ref<CommandRunOutput | null>(null);
let unlistenCommandOutput: UnlistenFn | null = null;
let unlistenCommandRunStarted: UnlistenFn | null = null;
let unlistenCommandRunFinished: UnlistenFn | null = null;
let unlistenCommandRunFailed: UnlistenFn | null = null;

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
  try {
    await Promise.all([refreshPathStatus(), refreshRegisteredCommands()]);
    pushToast("info", "命令列表和 PATH 状态已刷新。");
  } finally {
    isRefreshing.value = false;
  }
}

async function registerCommand() {
  if (!canRegister.value) return;

  const name = commandName.value.trim();
  isRegistering.value = true;
  try {
    const registeredCommand = await invoke<RegisteredCommand>("register_command", {
      scriptPath: scriptPath.value.trim(),
      commandName: name,
    });
    commands.value = [
      registeredCommand,
      ...commands.value.filter((command) => command.name !== registeredCommand.name),
    ];
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

function appendCommandOutput(chunk: CommandOutputChunk) {
  if (chunk.commandName !== runningCommandName.value) return;

  if (!runOutput.value || runOutput.value.commandName !== chunk.commandName) {
    runOutput.value = {
      commandName: chunk.commandName,
      exitCode: null,
      stdout: "",
      stderr: "",
      status: "running",
    };
  }

  if (chunk.stream === "stdout") {
    runOutput.value.stdout += chunk.text;
  } else {
    runOutput.value.stderr += chunk.text;
  }
}

function startCommandRun(event: CommandRunStarted) {
  activeModule.value = "commands";
  runningCommandName.value = event.commandName;
  runOutput.value = {
    commandName: event.commandName,
    exitCode: null,
    stdout: "",
    stderr: "",
    status: "running",
  };
}

function finishCommandRun(result: CommandRunFinished) {
  activeModule.value = "commands";
  runningCommandName.value = null;
  runOutput.value = {
    ...result,
    status: result.exitCode === 0 ? "success" : "failed",
  };

  if (result.exitCode === 0) {
    pushToast("success", `${result.commandName} 执行完成。`);
  } else {
    pushToast("error", `${result.commandName} 执行失败，退出码：${result.exitCode ?? "未知"}。`);
  }
}

function failCommandRun(event: CommandRunFailed) {
  activeModule.value = "commands";
  runningCommandName.value = null;
  runOutput.value = {
    commandName: event.commandName,
    exitCode: null,
    stdout: "",
    stderr: event.message,
    status: "failed",
  };
  pushToast("error", `${event.commandName} 执行失败。`);
}

async function runCommand(command: RegisteredCommand) {
  if (runningCommandName.value) return;

  runningCommandName.value = command.name;
  runOutput.value = {
    commandName: command.name,
    exitCode: null,
    stdout: "",
    stderr: "",
    status: "running",
  };

  try {
    const result = await invoke<CommandRunResult>("run_registered_command", {
      commandName: command.name,
    });
    runOutput.value = {
      ...result,
      status: result.exitCode === 0 ? "success" : "failed",
    };

    if (result.exitCode === 0) {
      pushToast("success", `${command.name} 执行完成。`);
    } else {
      pushToast("error", `${command.name} 执行失败，退出码：${result.exitCode ?? "未知"}。`);
    }
  } catch (error) {
    pushToast("error", `执行失败：${String(error)}`);
  } finally {
    runningCommandName.value = null;
  }
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
  void listen<CommandOutputChunk>("command-output", (event) => {
    appendCommandOutput(event.payload);
  })
    .then((unlisten) => {
      unlistenCommandOutput = unlisten;
    })
    .catch((error) => {
      pushToast("error", `监听命令输出失败：${String(error)}`);
    });
  void listen<CommandRunStarted>("command-run-started", (event) => {
    startCommandRun(event.payload);
  })
    .then((unlisten) => {
      unlistenCommandRunStarted = unlisten;
    })
    .catch((error) => {
      pushToast("error", `监听命令开始事件失败：${String(error)}`);
    });
  void listen<CommandRunFinished>("command-run-finished", (event) => {
    finishCommandRun(event.payload);
  })
    .then((unlisten) => {
      unlistenCommandRunFinished = unlisten;
    })
    .catch((error) => {
      pushToast("error", `监听命令完成事件失败：${String(error)}`);
    });
  void listen<CommandRunFailed>("command-run-failed", (event) => {
    failCommandRun(event.payload);
  })
    .then((unlisten) => {
      unlistenCommandRunFailed = unlisten;
    })
    .catch((error) => {
      pushToast("error", `监听命令失败事件失败：${String(error)}`);
    });
});

onBeforeUnmount(() => {
  unlistenCommandOutput?.();
  unlistenCommandRunStarted?.();
  unlistenCommandRunFinished?.();
  unlistenCommandRunFailed?.();
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
          :running-command-name="runningCommandName"
          :run-output="runOutput"
          @register="registerCommand"
          @refresh="refreshCommands"
          @copy-path="copyPathCommand"
          @run-command="runCommand"
          @reveal-command="revealCommand"
          @request-delete="commandToDelete = $event"
        />
        <AdbView v-else-if="activeModule === 'adb'" />
        <UpdaterView v-else />
      </el-main>
    </el-container>

    <ConfirmDialog :command="commandToDelete" @cancel="commandToDelete = null" @confirm="deleteCommand" />
  </el-container>
</template>
