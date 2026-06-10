export type UpdateDownloadEvent =
  | { event: "Started"; data: { contentLength?: number } }
  | { event: "Progress"; data: { chunkLength: number } }
  | { event: "Finished" };

export type UpdatePackage = {
  version: string;
  currentVersion: string;
  body?: string;
  date?: string;
  downloadAndInstall: (onEvent?: (event: UpdateDownloadEvent) => void) => Promise<void>;
};

export type UpdaterDependencies = {
  check: () => Promise<UpdatePackage | null>;
  relaunch: () => Promise<void>;
};

export type UpdateCheckResult =
  | {
      available: false;
      message: string;
    }
  | {
      available: true;
      version: string;
      currentVersion: string;
      notes?: string;
      date?: string;
      message: string;
    };

export async function checkForUpdate(dependencies: UpdaterDependencies): Promise<UpdateCheckResult> {
  const update = await dependencies.check();

  if (!update) {
    return {
      available: false,
      message: "当前已是最新版本。",
    };
  }

  return {
    available: true,
    version: update.version,
    currentVersion: update.currentVersion,
    notes: update.body,
    date: update.date,
    message: `发现新版本 ${update.version}。`,
  };
}

export async function installUpdate(
  update: Pick<UpdatePackage, "downloadAndInstall">,
  dependencies: Pick<UpdaterDependencies, "relaunch">,
  onProgress: (message: string) => void,
): Promise<void> {
  let contentLength = 0;
  let downloaded = 0;

  await update.downloadAndInstall((event) => {
    if (event.event === "Started") {
      contentLength = event.data.contentLength ?? 0;
      downloaded = 0;
      onProgress("开始下载更新包...");
      return;
    }

    if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      if (contentLength > 0) {
        const percent = Math.min(100, Math.round((downloaded / contentLength) * 100));
        onProgress(`正在下载更新包：${percent}%`);
      } else {
        onProgress("正在下载更新包...");
      }
      return;
    }

    onProgress("更新包安装完成，正在重启应用...");
  });

  await dependencies.relaunch();
}
