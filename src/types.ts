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
};

export type RegisteredCommand = {
  name: string;
  scriptPath: string;
  entryPath: string;
  entryType: EntryType;
  createdAt: string;
};
