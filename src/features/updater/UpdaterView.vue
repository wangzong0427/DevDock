<script setup lang="ts">
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { Download, Refresh } from "@element-plus/icons-vue";
import { ElMessage } from "element-plus";
import { computed, ref } from "vue";
import {
  checkForUpdate,
  installUpdate,
  keepUpdatePackageRaw,
  type UpdateCheckResult,
  type UpdatePackage,
} from "./updateService";

const isChecking = ref(false);
const isInstalling = ref(false);
const checkResult = ref<UpdateCheckResult | null>(null);
const availableUpdate = ref<UpdatePackage | null>(null);
const progressMessage = ref("");

const statusTitle = computed(() => {
  if (isInstalling.value) return "正在安装更新";
  if (isChecking.value) return "正在检查更新";
  if (!checkResult.value) return "尚未检查更新";
  if (checkResult.value.available) return "发现可用更新";
  return "当前已是最新版本";
});

const statusDescription = computed(() => {
  if (progressMessage.value) return progressMessage.value;
  if (!checkResult.value) return "从 GitHub Release 拉取最新版本信息。";
  return checkResult.value.message;
});

function showMessage(type: "success" | "error" | "info", message: string) {
  ElMessage({
    type,
    message,
    duration: 2600,
    showClose: true,
  });
}

async function checkUpdates() {
  isChecking.value = true;
  progressMessage.value = "";
  availableUpdate.value = null;

  try {
    let checkedUpdate: UpdatePackage | null = null;
    const result = await checkForUpdate({
      check: async () => {
        checkedUpdate = await check();
        return checkedUpdate;
      },
      relaunch,
    });

    checkResult.value = result;
    availableUpdate.value = keepUpdatePackageRaw(checkedUpdate);
    showMessage(result.available ? "success" : "info", result.message);
  } catch (error) {
    checkResult.value = null;
    showMessage("error", `检查更新失败：${String(error)}`);
  } finally {
    isChecking.value = false;
  }
}

async function downloadAndInstall() {
  if (!availableUpdate.value) return;

  isInstalling.value = true;
  progressMessage.value = "准备下载更新包...";

  try {
    await installUpdate(availableUpdate.value, { relaunch }, (message) => {
      progressMessage.value = message;
    });
  } catch (error) {
    showMessage("error", `安装更新失败：${String(error)}`);
    progressMessage.value = "";
    isInstalling.value = false;
  }
}
</script>

<template>
  <section class="updates-view" aria-labelledby="updates-title">
    <header class="page-header">
      <div>
        <h2 id="updates-title">更新</h2>
        <p>从 GitHub Release 获取最新软件包，并在下载完成后安装更新。</p>
      </div>
      <el-button type="primary" plain :icon="Refresh" :loading="isChecking" @click="checkUpdates">
        检查更新
      </el-button>
    </header>

    <el-card class="update-card" shadow="never">
      <template #header>
        <div class="card-header">
          <h3>{{ statusTitle }}</h3>
          <el-tag v-if="checkResult?.available" type="success" round>可更新</el-tag>
          <el-tag v-else-if="checkResult" type="info" round>最新</el-tag>
        </div>
      </template>

      <div class="update-content">
        <p class="update-message">{{ statusDescription }}</p>

        <dl v-if="checkResult?.available" class="update-meta">
          <div>
            <dt>当前版本</dt>
            <dd>{{ checkResult.currentVersion }}</dd>
          </div>
          <div>
            <dt>最新版本</dt>
            <dd>{{ checkResult.version }}</dd>
          </div>
          <div v-if="checkResult.date">
            <dt>发布时间</dt>
            <dd>{{ checkResult.date }}</dd>
          </div>
        </dl>

        <el-alert
          v-if="checkResult?.available && checkResult.notes"
          class="update-notes"
          type="info"
          :closable="false"
          :title="checkResult.notes"
        />

        <div class="update-actions">
          <el-button
            type="primary"
            :icon="Download"
            :disabled="!availableUpdate || isChecking"
            :loading="isInstalling"
            @click="downloadAndInstall"
          >
            下载并安装
          </el-button>
        </div>
      </div>
    </el-card>
  </section>
</template>
