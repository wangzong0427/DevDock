import { describe, expect, it, vi } from "vitest";
import {
  checkForUpdate,
  installUpdate,
  type UpdateDownloadEvent,
  type UpdatePackage,
  type UpdaterDependencies,
} from "./updateService";

function createDependencies(update: UpdatePackage | null): UpdaterDependencies {
  return {
    check: vi.fn(async () => update),
    relaunch: vi.fn(async () => undefined),
  };
}

describe("updateService", () => {
  it("returns an idle result when no update is available", async () => {
    const dependencies = createDependencies(null);

    const result = await checkForUpdate(dependencies);

    expect(result).toEqual({
      available: false,
      message: "当前已是最新版本。",
    });
    expect(dependencies.check).toHaveBeenCalledOnce();
  });

  it("returns version details when an update is available", async () => {
    const dependencies = createDependencies({
      version: "0.2.0",
      currentVersion: "0.1.0",
      body: "修复命令执行问题",
      date: "2026-06-09T12:00:00Z",
      downloadAndInstall: vi.fn(),
    });

    const result = await checkForUpdate(dependencies);

    expect(result).toEqual({
      available: true,
      version: "0.2.0",
      currentVersion: "0.1.0",
      notes: "修复命令执行问题",
      date: "2026-06-09T12:00:00Z",
      message: "发现新版本 0.2.0。",
    });
  });

  it("downloads, installs and relaunches after install finishes", async () => {
    const downloadAndInstall = vi.fn(async (onEvent?: (event: UpdateDownloadEvent) => void) => {
      if (!onEvent) return;
      onEvent({ event: "Started", data: { contentLength: 100 } });
      onEvent({ event: "Progress", data: { chunkLength: 25 } });
      onEvent({ event: "Finished" });
    });
    const dependencies = {
      check: vi.fn(async () => null),
      relaunch: vi.fn(async () => undefined),
    };
    const progressMessages: string[] = [];

    await installUpdate(
      { downloadAndInstall },
      dependencies,
      (message) => {
        progressMessages.push(message);
      },
    );

    expect(downloadAndInstall).toHaveBeenCalledOnce();
    expect(dependencies.relaunch).toHaveBeenCalledOnce();
    expect(progressMessages).toEqual([
      "开始下载更新包...",
      "正在下载更新包：25%",
      "更新包安装完成，正在重启应用...",
    ]);
  });
});
