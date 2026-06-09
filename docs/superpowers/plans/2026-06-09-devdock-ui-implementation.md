# DevDock UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first DevDock Workbench UI in the existing Tauri + Vue app, focused on the Commands module with PATH status, registration form, command list, delete confirmation, and an ADB planned entry.

**Architecture:** Keep the first implementation in `src/App.vue` because the current app is still the default scaffold and has no component structure yet. Use typed local mock data and UI state that mirrors the future Tauri command contract, so the next backend phase can replace mock functions with `invoke(...)` calls without redesigning the screen.

**Tech Stack:** Vue 3 `<script setup>`, TypeScript, Vite, Tauri v2 frontend package, scoped/global CSS in `App.vue`.

---

## Scope

This plan implements the UI only. It does not implement Rust command registration, real file picking, real PATH detection, real command deletion, or ADB behavior. The UI should be interactive with local mock state so layout, validation, empty states, delete confirmation, and toast behavior can be verified before backend work starts.

## File Structure

- Modify `src/App.vue`
  - Replace the default Tauri/Vite/Vue welcome screen.
  - Define frontend types for `PathStatus`, `RegisteredCommand`, `PlatformInfo`, `ToastMessage`, and active module state.
  - Render the Workbench shell, sidebar, Commands screen, ADB planned screen, confirm dialog, and toast.
  - Add local mock handlers for register, delete, reveal, refresh, copy PATH guidance, and module switching.
  - Add all first-pass styling.
- Leave `src/main.ts` unchanged.
  - It already mounts `App.vue`.
- Do not modify `package.json`.
  - The first UI pass avoids adding icon or UI dependencies.
- Do not modify `src-tauri` in this plan.
  - Backend implementation should be planned separately after the UI is accepted.

## Verification Commands

- Build frontend: `npm run build`
  - Expected: `vue-tsc --noEmit && vite build` completes successfully.
- Optional development server: `npm run dev -- --host 127.0.0.1`
  - Expected: Vite prints a localhost URL and the app renders the DevDock UI.
- Git commit is currently unavailable because `/Users/wangqinghua/learn/rust/DevDock` is not a Git repository. Do not include commit commands until a repository is initialized.

---

### Task 1: Replace Template State With DevDock UI State

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Remove the template imports and greeting state**

Delete this block from `src/App.vue`:

```vue
<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const greetMsg = ref("");
const name = ref("");

async function greet() {
  // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
  greetMsg.value = await invoke("greet", { name: name.value });
}
</script>
```

- [ ] **Step 2: Add DevDock typed state**

Insert this replacement `<script setup>` block at the top of `src/App.vue`:

```vue
<script setup lang="ts">
import { computed, ref } from "vue";

type ActiveModule = "commands" | "adb";
type PathState = "checking" | "ok" | "missing" | "error";
type EntryType = "symlink" | "wrapper" | "cmd-shim" | "ps1-shim";

type PlatformInfo = {
  name: "macOS" | "Linux" | "Windows";
};

type PathStatus = {
  state: PathState;
  binDir: string;
  message?: string;
  suggestedCommand?: string;
};

type RegisteredCommand = {
  name: string;
  scriptPath: string;
  entryPath: string;
  entryType: EntryType;
  createdAt: string;
};

type ToastMessage = {
  id: number;
  tone: "success" | "error" | "info";
  text: string;
};

const activeModule = ref<ActiveModule>("commands");
const platformInfo = ref<PlatformInfo>({ name: "macOS" });
const pathStatus = ref<PathStatus>({
  state: "missing",
  binDir: "~/.local/bin",
  message: "Command directory is not on PATH.",
  suggestedCommand: "export PATH=\"$HOME/.local/bin:$PATH\"",
});

const scriptPath = ref("");
const commandName = ref("");
const isRegistering = ref(false);
const isRefreshing = ref(false);
const commandToDelete = ref<RegisteredCommand | null>(null);
const toastMessages = ref<ToastMessage[]>([]);

const commands = ref<RegisteredCommand[]>([
  {
    name: "sync-env",
    scriptPath: "/Users/me/dev/scripts/sync-env.sh",
    entryPath: "~/.local/bin/sync-env",
    entryType: "wrapper",
    createdAt: "2026-06-09 10:18",
  },
  {
    name: "clean-cache",
    scriptPath: "/Users/me/dev/scripts/clean-cache.sh",
    entryPath: "~/.local/bin/clean-cache",
    entryType: "symlink",
    createdAt: "2026-06-09 11:02",
  },
]);

const commandNamePattern = /^[A-Za-z0-9_.-]+$/;

const commandNameError = computed(() => {
  if (!commandName.value) return "";
  if (!commandNamePattern.test(commandName.value)) {
    return "Use letters, numbers, dot, dash, or underscore.";
  }
  if (commands.value.some((command) => command.name === commandName.value)) {
    return "A command with this name already exists.";
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
</script>
```

- [ ] **Step 3: Run type check and confirm expected template errors**

Run:

```bash
npm run build
```

Expected: FAIL because the old template still references `greet`, `name`, and `greetMsg`. This confirms the script replacement happened and the template still needs to be replaced.

---

### Task 2: Build the Workbench Template

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Replace the default `<template>`**

Replace the existing `<template>...</template>` block with:

```vue
<template>
  <div class="app-shell">
    <aside class="sidebar" aria-label="DevDock navigation">
      <div class="brand">
        <div class="brand-mark">DD</div>
        <div>
          <h1>DevDock</h1>
          <p>Local developer commands</p>
        </div>
      </div>

      <nav class="nav-list">
        <button
          class="nav-item"
          :class="{ active: activeModule === 'commands' }"
          type="button"
          @click="activeModule = 'commands'"
        >
          <span class="nav-icon">⌘</span>
          <span>Commands</span>
        </button>
        <button
          class="nav-item planned"
          :class="{ active: activeModule === 'adb' }"
          type="button"
          @click="activeModule = 'adb'"
        >
          <span class="nav-icon">▣</span>
          <span>ADB</span>
          <span class="planned-chip">Planned</span>
        </button>
      </nav>

      <div class="sidebar-footer">
        <div class="status-row">
          <span>Platform</span>
          <strong>{{ platformInfo.name }}</strong>
        </div>
        <div class="status-row">
          <span>PATH</span>
          <strong class="status-pill" :class="pathTone">
            {{ pathStatus.state === "ok" ? "OK" : pathStatus.state === "missing" ? "Missing" : "Checking" }}
          </strong>
        </div>
      </div>
    </aside>

    <main class="workspace">
      <section v-if="activeModule === 'commands'" class="commands-view" aria-labelledby="commands-title">
        <header class="page-header">
          <div>
            <h2 id="commands-title">Commands</h2>
            <p>Register local scripts as commands you can run from your terminal.</p>
          </div>
          <button class="secondary-button" type="button" @click="refreshCommands">
            {{ isRefreshing ? "Refreshing..." : "Refresh" }}
          </button>
        </header>

        <section class="panel register-panel" aria-labelledby="register-title">
          <div class="section-heading">
            <div>
              <h3 id="register-title">Register command</h3>
              <p>Create a terminal entry that points to a local script.</p>
            </div>
          </div>

          <div class="form-grid">
            <label class="field span-2">
              <span>Script file</span>
              <div class="file-picker-row">
                <input
                  v-model="scriptPath"
                  readonly
                  placeholder="Choose a script file"
                  aria-label="Selected script file"
                />
                <button class="secondary-button" type="button" @click="chooseMockScript">Browse</button>
              </div>
            </label>

            <label class="field">
              <span>Command name</span>
              <input
                v-model.trim="commandName"
                placeholder="my-command"
                aria-label="Command name"
              />
              <small v-if="commandNameError" class="field-error">{{ commandNameError }}</small>
            </label>

            <div class="preview-box">
              <span>Generated entry</span>
              <code>{{ entryPreview }}</code>
            </div>
          </div>

          <div class="panel-actions">
            <button
              class="primary-button"
              type="button"
              :disabled="!canRegister || isRegistering"
              @click="registerCommand"
            >
              {{ isRegistering ? "Registering..." : "Register command" }}
            </button>
          </div>
        </section>

        <section class="panel path-panel" :class="pathTone" aria-labelledby="path-title">
          <div>
            <h3 id="path-title">PATH status</h3>
            <p>{{ pathStatus.message }}</p>
          </div>
          <div class="path-details">
            <code>{{ pathStatus.binDir }}</code>
            <button
              v-if="pathStatus.suggestedCommand"
              class="secondary-button"
              type="button"
              @click="copyPathCommand"
            >
              Copy fix
            </button>
          </div>
        </section>

        <section class="panel command-list-panel" aria-labelledby="list-title">
          <div class="section-heading">
            <div>
              <h3 id="list-title">Registered commands</h3>
              <p>Generated entries managed by DevDock. Original scripts are never deleted.</p>
            </div>
            <span class="count-pill">{{ commands.length }}</span>
          </div>

          <div v-if="commands.length === 0" class="empty-state">
            <h4>No commands registered</h4>
            <p>Choose a script and create your first terminal command.</p>
          </div>

          <div v-else class="command-table" role="table" aria-label="Registered commands">
            <div class="table-row table-head" role="row">
              <span role="columnheader">Command</span>
              <span role="columnheader">Source script</span>
              <span role="columnheader">Entry</span>
              <span role="columnheader">Created</span>
              <span role="columnheader">Actions</span>
            </div>
            <div v-for="command in commands" :key="command.name" class="table-row" role="row">
              <strong role="cell">{{ command.name }}</strong>
              <span role="cell" class="truncate" :title="command.scriptPath">{{ command.scriptPath }}</span>
              <span role="cell">
                <span class="entry-type">{{ command.entryType }}</span>
              </span>
              <span role="cell">{{ command.createdAt }}</span>
              <span role="cell" class="row-actions">
                <button class="text-button" type="button" @click="revealCommand(command)">Reveal</button>
                <button class="text-button danger" type="button" @click="commandToDelete = command">Delete</button>
              </span>
            </div>
          </div>
        </section>
      </section>

      <section v-else class="adb-view panel" aria-labelledby="adb-title">
        <span class="planned-chip">Planned</span>
        <h2 id="adb-title">ADB</h2>
        <p>Device workflows will live here in a later version.</p>
      </section>
    </main>

    <div v-if="commandToDelete" class="dialog-backdrop" role="presentation">
      <section class="confirm-dialog" role="dialog" aria-modal="true" aria-labelledby="delete-title">
        <h3 id="delete-title">Delete command?</h3>
        <p>
          Delete the generated entry for
          <strong>{{ commandToDelete.name }}</strong>.
          The original script file will not be removed.
        </p>
        <div class="dialog-actions">
          <button class="secondary-button" type="button" @click="commandToDelete = null">Cancel</button>
          <button class="danger-button" type="button" @click="deleteCommand">Delete</button>
        </div>
      </section>
    </div>

    <div class="toast-region" aria-live="polite" aria-label="Notifications">
      <div v-for="toast in toastMessages" :key="toast.id" class="toast" :class="toast.tone">
        {{ toast.text }}
      </div>
    </div>
  </div>
</template>
```

- [ ] **Step 2: Run build and confirm missing handler errors**

Run:

```bash
npm run build
```

Expected: FAIL with errors for undefined handlers such as `refreshCommands`, `chooseMockScript`, `registerCommand`, `copyPathCommand`, `revealCommand`, and `deleteCommand`.

---

### Task 3: Add Local UI Handlers

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Add helper functions before `</script>`**

Insert this code after the `pathTone` computed property and before `</script>`:

```ts
function pushToast(tone: ToastMessage["tone"], text: string) {
  const id = Date.now();
  toastMessages.value.push({ id, tone, text });
  window.setTimeout(() => {
    toastMessages.value = toastMessages.value.filter((toast) => toast.id !== id);
  }, 2600);
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
    pushToast("info", "Commands refreshed.");
  }, 500);
}

function registerCommand() {
  if (!canRegister.value) return;

  isRegistering.value = true;
  window.setTimeout(() => {
    const name = commandName.value;
    commands.value = [
      {
        name,
        scriptPath: scriptPath.value,
        entryPath: entryPreview.value,
        entryType: platformInfo.value.name === "Windows" ? "cmd-shim" : "wrapper",
        createdAt: new Date().toLocaleString([], {
          year: "numeric",
          month: "2-digit",
          day: "2-digit",
          hour: "2-digit",
          minute: "2-digit",
        }),
      },
      ...commands.value,
    ];
    scriptPath.value = "";
    commandName.value = "";
    isRegistering.value = false;
    pushToast("success", `Registered ${name}.`);
  }, 650);
}

async function copyPathCommand() {
  const command = pathStatus.value.suggestedCommand || pathStatus.value.binDir;
  try {
    await navigator.clipboard.writeText(command);
    pushToast("success", "PATH command copied.");
  } catch {
    pushToast("error", "Copy failed. Select the command manually.");
  }
}

function revealCommand(command: RegisteredCommand) {
  pushToast("info", `Reveal will open ${command.entryPath} after backend wiring.`);
}

function deleteCommand() {
  if (!commandToDelete.value) return;
  const deletedName = commandToDelete.value.name;
  commands.value = commands.value.filter((command) => command.name !== deletedName);
  commandToDelete.value = null;
  pushToast("success", `Deleted ${deletedName}.`);
}
```

- [ ] **Step 2: Run build and confirm CSS is still pending but TypeScript passes**

Run:

```bash
npm run build
```

Expected: PASS for TypeScript if the existing old CSS does not contain syntax issues. The page may still look wrong until Task 4 replaces the styles.

---

### Task 4: Replace Template Styles With DevDock Visual System

**Files:**
- Modify: `src/App.vue`

- [ ] **Step 1: Delete both old style blocks**

Remove the template style blocks that start with:

```vue
<style scoped>
.logo.vite:hover {
```

and:

```vue
<style>
:root {
```

- [ ] **Step 2: Add the DevDock style block**

Append this style block to the end of `src/App.vue`:

```vue
<style>
:root {
  font-family:
    Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI",
    sans-serif;
  color: #172026;
  background: #eef1f3;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
  min-width: 960px;
  min-height: 640px;
  background: #eef1f3;
}

button,
input {
  font: inherit;
}

button {
  cursor: pointer;
}

button:disabled {
  cursor: not-allowed;
  opacity: 0.58;
}

.app-shell {
  display: grid;
  grid-template-columns: 224px minmax(0, 1fr);
  min-height: 100vh;
  background: #eef1f3;
}

.sidebar {
  display: flex;
  flex-direction: column;
  gap: 28px;
  padding: 22px 14px;
  border-right: 1px solid #d8dee3;
  background: #e4e9ed;
}

.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 8px;
}

.brand-mark {
  display: grid;
  width: 38px;
  height: 38px;
  place-items: center;
  border: 1px solid #b8c7cd;
  border-radius: 8px;
  color: #0f766e;
  background: #f8fafb;
  font-size: 13px;
  font-weight: 800;
}

.brand h1,
.brand p,
.page-header h2,
.page-header p,
.section-heading h3,
.section-heading p,
.adb-view h2,
.adb-view p,
.confirm-dialog h3,
.confirm-dialog p {
  margin: 0;
}

.brand h1 {
  font-size: 18px;
  line-height: 1.2;
}

.brand p {
  margin-top: 2px;
  color: #60717b;
  font-size: 12px;
}

.nav-list {
  display: grid;
  gap: 6px;
}

.nav-item {
  display: grid;
  grid-template-columns: 22px 1fr auto;
  align-items: center;
  width: 100%;
  min-height: 38px;
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 0 10px;
  color: #34444d;
  background: transparent;
  text-align: left;
}

.nav-item:hover {
  background: #edf2f4;
}

.nav-item.active {
  border-color: #bdd4d7;
  color: #0b4f4a;
  background: #f7fbfb;
  box-shadow: inset 3px 0 0 #0f766e;
}

.nav-item.planned {
  color: #60717b;
}

.nav-icon {
  color: #75868f;
  font-size: 14px;
}

.planned-chip,
.count-pill,
.entry-type,
.status-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border-radius: 999px;
  font-size: 11px;
  font-weight: 700;
  line-height: 1;
  white-space: nowrap;
}

.planned-chip {
  padding: 5px 7px;
  color: #6a5a16;
  background: #fff2bd;
}

.sidebar-footer {
  display: grid;
  gap: 8px;
  margin-top: auto;
  padding: 12px;
  border: 1px solid #d2dbe0;
  border-radius: 8px;
  background: #edf2f4;
}

.status-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  color: #60717b;
  font-size: 12px;
}

.status-row strong {
  color: #26343b;
  font-size: 12px;
}

.status-pill {
  padding: 5px 7px;
}

.status-pill.ok,
.path-panel.ok {
  color: #0f5f48;
  background: #e4f6ec;
}

.status-pill.warning,
.path-panel.warning {
  color: #74510f;
  background: #fff5d7;
}

.status-pill.danger,
.path-panel.danger {
  color: #8f1d1d;
  background: #fde8e8;
}

.status-pill.muted {
  color: #50616b;
  background: #dfe6ea;
}

.workspace {
  min-width: 0;
  padding: 30px 32px;
}

.commands-view {
  display: grid;
  gap: 18px;
}

.page-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}

.page-header h2 {
  font-size: 24px;
  line-height: 1.2;
}

.page-header p,
.section-heading p,
.adb-view p {
  margin-top: 5px;
  color: #60717b;
  font-size: 13px;
}

.panel {
  border: 1px solid #d8dee3;
  border-radius: 8px;
  background: #fbfcfc;
  box-shadow: 0 1px 2px rgb(23 32 38 / 4%);
}

.register-panel,
.command-list-panel {
  padding: 18px;
}

.section-heading {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 16px;
}

.section-heading h3,
.path-panel h3 {
  color: #1d2b32;
  font-size: 15px;
  line-height: 1.3;
}

.form-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(220px, 0.46fr);
  gap: 14px;
}

.span-2 {
  grid-column: 1 / -1;
}

.field {
  display: grid;
  gap: 7px;
}

.field > span,
.preview-box > span {
  color: #42525b;
  font-size: 12px;
  font-weight: 700;
}

input {
  width: 100%;
  height: 38px;
  border: 1px solid #ccd6dc;
  border-radius: 7px;
  padding: 0 11px;
  color: #1b2a32;
  background: #ffffff;
  outline: none;
}

input:focus {
  border-color: #0f766e;
  box-shadow: 0 0 0 3px rgb(15 118 110 / 14%);
}

input::placeholder {
  color: #94a2aa;
}

.file-picker-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
}

.field-error {
  color: #b42318;
  font-size: 12px;
}

.preview-box {
  display: grid;
  align-content: start;
  gap: 7px;
  min-width: 0;
}

code {
  display: inline-flex;
  align-items: center;
  min-height: 38px;
  max-width: 100%;
  overflow: hidden;
  border: 1px solid #d7e0e5;
  border-radius: 7px;
  padding: 0 10px;
  color: #1d3737;
  background: #f4f7f8;
  font-family: "SFMono-Regular", Consolas, "Liberation Mono", monospace;
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.panel-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.primary-button,
.secondary-button,
.danger-button {
  min-height: 36px;
  border-radius: 7px;
  padding: 0 13px;
  font-size: 13px;
  font-weight: 700;
}

.primary-button {
  border: 1px solid #0f766e;
  color: #ffffff;
  background: #0f766e;
}

.primary-button:hover:not(:disabled) {
  background: #0b615a;
}

.secondary-button {
  border: 1px solid #cbd7dd;
  color: #2f414a;
  background: #ffffff;
}

.secondary-button:hover {
  border-color: #aebdc5;
  background: #f5f8f9;
}

.danger-button {
  border: 1px solid #b42318;
  color: #ffffff;
  background: #b42318;
}

.text-button {
  border: 0;
  padding: 0;
  color: #0f766e;
  background: transparent;
  font-size: 12px;
  font-weight: 700;
}

.text-button.danger {
  color: #b42318;
}

.path-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 15px 16px;
  border-color: transparent;
}

.path-panel p {
  margin: 4px 0 0;
  font-size: 13px;
}

.path-details {
  display: flex;
  align-items: center;
  min-width: 0;
  gap: 8px;
}

.count-pill {
  min-width: 28px;
  height: 24px;
  color: #315058;
  background: #e6eef1;
}

.empty-state {
  display: grid;
  gap: 5px;
  border: 1px dashed #cbd7dd;
  border-radius: 8px;
  padding: 24px;
  text-align: center;
}

.empty-state h4,
.empty-state p {
  margin: 0;
}

.empty-state h4 {
  font-size: 15px;
}

.empty-state p {
  color: #60717b;
  font-size: 13px;
}

.command-table {
  display: grid;
  border: 1px solid #d8dee3;
  border-radius: 8px;
  overflow: hidden;
}

.table-row {
  display: grid;
  grid-template-columns: minmax(120px, 0.8fr) minmax(190px, 1.4fr) 110px 140px 118px;
  align-items: center;
  gap: 14px;
  min-height: 48px;
  border-top: 1px solid #e3e8eb;
  padding: 0 12px;
  color: #33444d;
  font-size: 13px;
}

.table-row:first-child {
  border-top: 0;
}

.table-head {
  min-height: 38px;
  color: #60717b;
  background: #f3f6f7;
  font-size: 11px;
  font-weight: 800;
  text-transform: uppercase;
}

.truncate {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.entry-type {
  justify-self: start;
  padding: 5px 7px;
  color: #315058;
  background: #e6eef1;
}

.row-actions {
  display: flex;
  gap: 10px;
}

.adb-view {
  max-width: 560px;
  padding: 22px;
}

.adb-view h2 {
  margin-top: 14px;
  font-size: 24px;
}

.dialog-backdrop {
  position: fixed;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgb(15 23 42 / 28%);
}

.confirm-dialog {
  width: min(420px, 100%);
  border: 1px solid #d8dee3;
  border-radius: 8px;
  padding: 20px;
  background: #ffffff;
  box-shadow: 0 24px 48px rgb(15 23 42 / 18%);
}

.confirm-dialog h3 {
  font-size: 17px;
}

.confirm-dialog p {
  margin-top: 10px;
  color: #4f6069;
  font-size: 13px;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 18px;
}

.toast-region {
  position: fixed;
  right: 18px;
  bottom: 18px;
  display: grid;
  gap: 8px;
  width: min(340px, calc(100vw - 36px));
}

.toast {
  border: 1px solid #d8dee3;
  border-radius: 8px;
  padding: 11px 12px;
  background: #ffffff;
  box-shadow: 0 12px 30px rgb(15 23 42 / 12%);
  font-size: 13px;
  font-weight: 700;
}

.toast.success {
  border-color: #b7dfc8;
  color: #0f5f48;
}

.toast.error {
  border-color: #f2b8b5;
  color: #9f2018;
}

.toast.info {
  border-color: #b9d5dd;
  color: #245462;
}

@media (max-width: 820px) {
  body {
    min-width: 720px;
  }

  .app-shell {
    grid-template-columns: 184px minmax(0, 1fr);
  }

  .workspace {
    padding: 22px;
  }

  .form-grid,
  .table-row {
    grid-template-columns: 1fr;
  }

  .path-panel,
  .page-header {
    align-items: stretch;
    flex-direction: column;
  }
}
</style>
```

- [ ] **Step 3: Run build**

Run:

```bash
npm run build
```

Expected: PASS.

---

### Task 5: Verify Required UI States

**Files:**
- Modify only if visual/state issues are found: `src/App.vue`

- [ ] **Step 1: Start the local Vite server**

Run:

```bash
npm run dev -- --host 127.0.0.1
```

Expected: Vite prints a local URL such as `http://127.0.0.1:5173/`.

- [ ] **Step 2: Verify initial screen**

Open the Vite URL in the available browser tool or regular browser.

Expected:

- Left sidebar shows `DevDock`.
- `Commands` is active.
- `ADB` appears with `Planned`.
- Sidebar footer shows platform and PATH status.
- Right side shows Commands header, register panel, PATH panel, and registered commands list.

- [ ] **Step 3: Verify registration happy path**

In the UI:

1. Click `Browse`.
2. Confirm the script field becomes `/Users/me/dev/scripts/deploy-preview.sh`.
3. Confirm command name becomes `deploy-preview`.
4. Click `Register command`.

Expected:

- Button changes to `Registering...`.
- New `deploy-preview` row appears at the top of the list.
- Script and command inputs clear.
- Toast shows `Registered deploy-preview.`

- [ ] **Step 4: Verify validation**

In the command name field, type:

```text
bad name
```

Expected:

- Inline error says `Use letters, numbers, dot, dash, or underscore.`
- `Register command` is disabled.

Then type:

```text
sync-env
```

Expected:

- Inline error says `A command with this name already exists.`
- `Register command` is disabled.

- [ ] **Step 5: Verify PATH copy**

Click `Copy fix`.

Expected:

- Toast shows either `PATH command copied.` or `Copy failed. Select the command manually.`
- No layout shift occurs.

- [ ] **Step 6: Verify delete confirmation**

Click `Delete` on a command row.

Expected:

- Confirmation dialog appears.
- Dialog names the command.
- Dialog explains the original script is not removed.

Click `Cancel`.

Expected:

- Dialog closes and command remains.

Click `Delete` again, then confirm.

Expected:

- Dialog closes.
- Command row disappears.
- Toast shows `Deleted <command>.`

- [ ] **Step 7: Verify ADB planned view**

Click `ADB` in the sidebar.

Expected:

- Right content changes to a simple ADB planned page.
- No fake device controls appear.

- [ ] **Step 8: Stop the dev server**

Stop the running dev server with `Ctrl+C`.

Expected:

- Terminal returns to the shell prompt.

---

### Task 6: Final Build Check

**Files:**
- Read: `src/App.vue`
- Read: `docs/superpowers/specs/2026-06-09-devdock-ui-design.md`

- [ ] **Step 1: Run production build**

Run:

```bash
npm run build
```

Expected: PASS.

- [ ] **Step 2: Scan for scaffold leftovers**

Run:

```bash
rg -n "Welcome to Tauri|greet|vite.svg|tauri.svg|vue.svg|Click on the Tauri" src
```

Expected: no matches.

- [ ] **Step 3: Confirm scope against spec**

Check these items manually:

- First screen is Commands workspace.
- Script selection, command name input, generated entry preview, and register action are visible.
- PATH status is visible without settings.
- Registered commands are visible and deletable.
- ADB exists only as a planned entry.
- The UI avoids a marketing landing page.
- No Rust backend behavior is claimed as real.

Expected: all items are true.

## Plan Self-Review

Spec coverage:

- Workbench side navigation: Task 2 and Task 4.
- Commands registration panel: Task 2, Task 3, Task 4, Task 5.
- PATH status panel: Task 2, Task 3, Task 4, Task 5.
- Command list and delete flow: Task 2, Task 3, Task 4, Task 5.
- ADB planned entry: Task 2, Task 4, Task 5.
- Quiet developer-tool visual style: Task 4.
- Vue 3 implementation boundary: Task 1 through Task 4.
- Backend non-goals: Scope section and Task 6.

Placeholder scan:

- The plan has been checked for unfinished markers and vague edge-case instructions.

Type consistency:

- `PathState`, `PathStatus`, `RegisteredCommand`, `EntryType`, and handler names are introduced in Task 1 or Task 3 before use.
- Template references in Task 2 match the state and functions defined in Task 1 and Task 3.
