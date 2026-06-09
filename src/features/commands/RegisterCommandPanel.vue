<script setup lang="ts">
import { FolderOpened, Plus } from "@element-plus/icons-vue";

defineProps<{
  scriptPath: string;
  commandName: string;
  commandNameError: string;
  entryPreview: string;
  canRegister: boolean;
  isRegistering: boolean;
}>();

const emit = defineEmits<{
  "update:scriptPath": [value: string];
  "update:commandName": [value: string];
  browse: [];
  register: [];
}>();

function updateScriptPath(value: string | number) {
  emit("update:scriptPath", String(value));
}

function updateCommandName(value: string | number) {
  emit("update:commandName", String(value).trim());
}
</script>

<template>
  <el-card class="register-card" shadow="never">
    <template #header>
      <div class="card-header">
        <h3 id="register-title">注册命令</h3>
      </div>
    </template>

    <el-form label-position="top" class="register-form" @submit.prevent>
      <el-form-item label="脚本文件">
        <el-input
          :model-value="scriptPath"
          readonly
          placeholder="选择一个脚本文件"
          @update:model-value="updateScriptPath"
        >
          <template #append>
            <el-button :icon="FolderOpened" @click="emit('browse')">选择</el-button>
          </template>
        </el-input>
      </el-form-item>

      <el-form-item label="命令名" :error="commandNameError">
        <el-input
          :model-value="commandName"
          clearable
          placeholder="my-command"
          @update:model-value="updateCommandName"
        />
      </el-form-item>

      <el-form-item label="生成入口">
        <el-input :model-value="entryPreview" readonly />
      </el-form-item>

      <div class="form-actions">
        <el-button
          type="primary"
          :icon="Plus"
          :disabled="!canRegister"
          :loading="isRegistering"
          @click="emit('register')"
        >
          注册命令
        </el-button>
      </div>
    </el-form>
  </el-card>
</template>
