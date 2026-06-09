export type ActiveModule = "commands" | "adb";
export type PathState = "checking" | "ok" | "missing" | "error";
export type EntryType = "symlink" | "wrapper" | "cmd-shim" | "ps1-shim";

export type PlatformInfo = {
  name: "macOS" | "Linux" | "Windows";
};

export type PathStatus = {
  state: PathState;
  binDir: string;
  message: string;
  suggestedCommand?: string;
  paths?: string[];
};

export type RegisteredCommand = {
  name: string;
  scriptPath: string;
  entryPath: string;
  entryType: EntryType;
  createdAt: string;
};

export type CommandRunResult = {
  commandName: string;
  exitCode?: number | null;
  stdout: string;
  stderr: string;
};

export type CommandOutputStream = "stdout" | "stderr";

export type CommandOutputChunk = {
  commandName: string;
  stream: CommandOutputStream;
  text: string;
};

export type CommandRunOutput = CommandRunResult & {
  status: "running" | "success" | "failed";
};
