# DevDock 更新发布流程

DevDock 使用 Tauri updater 从 GitHub Release 拉取最新软件包。应用内“更新”页面会请求：

```text
https://github.com/wangzong0427/DevDock/releases/latest/download/latest.json
```

`latest.json` 由 `.github/workflows/release.yml` 中的 `tauri-apps/tauri-action@v0` 生成并上传到 GitHub Release。

## 签名密钥

Tauri updater 要求更新包必须签名。公钥已经写入 `src-tauri/tauri.conf.json`，私钥必须保存在 GitHub Actions Secret 中。

当前生成的本机临时私钥路径：

```bash
/private/tmp/devdock-updater.key
```

配置 GitHub Secrets：

```bash
gh auth login -h github.com
gh secret set TAURI_SIGNING_PRIVATE_KEY < /private/tmp/devdock-updater.key
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --body ""
```

不要把私钥提交到 git。丢失私钥后，已经安装旧版本的用户将无法通过当前公钥验证后续更新包。

## 发布版本

`src-tauri/tauri.conf.json` 里的 `version` 必须和 tag 版本一致。tag 可以带 `v` 前缀。

示例：

```bash
git tag v0.1.0
git push origin v0.1.0
```

workflow 会校验：

- tag 指向的提交必须在 `origin/master` 上。
- tag 版本必须等于 `src-tauri/tauri.conf.json` 的版本。
- `TAURI_SIGNING_PRIVATE_KEY` 必须存在。

通过校验后，workflow 会构建 macOS、Windows 和 Linux 产物，创建或更新对应 tag 的 GitHub Release，并上传安装包、updater 包、签名文件和 `latest.json`。

## 本地验证

使用本机私钥内容验证 updater 签名产物生成：

```bash
TAURI_SIGNING_PRIVATE_KEY="$(cat /private/tmp/devdock-updater.key)" \
TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
npm run tauri build -- --bundles app
```

成功后会生成类似文件：

```text
src-tauri/target/release/bundle/macos/devdock.app.tar.gz
src-tauri/target/release/bundle/macos/devdock.app.tar.gz.sig
```
