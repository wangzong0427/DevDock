<script setup lang="ts">
import { Cellphone, Files, Monitor, Operation, Refresh, WarningFilled } from "@element-plus/icons-vue";
import type { ActiveModule, PlatformInfo } from "../types";

defineProps<{
  activeModule: ActiveModule;
  platformInfo: PlatformInfo;
  pathTone: string;
  pathStatusLabel: string;
}>();

const emit = defineEmits<{
  "update:activeModule": [value: ActiveModule];
}>();

function selectModule(index: string) {
  emit("update:activeModule", index as ActiveModule);
}
</script>

<template>
  <el-aside class="sidebar" aria-label="DevDock 导航">
    <div class="brand">
      <el-icon class="brand-mark" :size="28" aria-hidden="true"><Operation /></el-icon>
      <div>
        <h1>DevDock</h1>
        <p>本地开发命令</p>
      </div>
    </div>

    <el-menu :default-active="activeModule" class="sidebar-menu" @select="selectModule">
      <el-menu-item index="commands">
        <el-icon><Files /></el-icon>
        <span>命令</span>
      </el-menu-item>
      <el-menu-item index="adb">
        <el-icon><Cellphone /></el-icon>
        <span>ADB</span>
        <el-tag class="menu-tag" size="small" type="info" round>规划中</el-tag>
      </el-menu-item>
      <el-menu-item index="updater">
        <el-icon><Refresh /></el-icon>
        <span>更新</span>
      </el-menu-item>
    </el-menu>

    <div class="sidebar-footer">
      <div class="footer-row">
        <el-icon><Monitor /></el-icon>
        <strong>{{ platformInfo.name }}</strong>
      </div>
      <div class="footer-row" :class="pathTone">
        <el-icon><WarningFilled /></el-icon>
        <strong>{{ pathStatusLabel }}</strong>
      </div>
    </div>
  </el-aside>
</template>
