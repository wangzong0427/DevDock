# DevDock 更新发布流程

DevDock 使用 Tauri updater 从 GitHub Release 拉取最新软件包。应用内“更新”页面会请求：

```text
https://github.com/wangzong0427/DevDock/releases/latest/download/latest.json
```

`latest.json` 由 `.github/workflows/release.yml` 中的 `tauri-apps/tauri-action@v0` 生成并上传到 GitHub Release。

## Tauri updater 签名密钥

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

## macOS 安装签名

Tauri updater 签名只用于校验更新包完整性，不等于 Apple Gatekeeper 认可的 macOS 应用签名。

当前 `.github/workflows/release.yml` 在没有配置 Apple 签名证书时，会默认写入：

```bash
APPLE_SIGNING_IDENTITY=-
```

`-` 表示 ad-hoc 签名。它不需要 Apple Developer 账号，能避免 macOS 把应用当成完全未签名二进制，但不会让 GitHub 下载的 DMG 自动通过 Gatekeeper。用户首次打开时仍需要在“系统设置 > 隐私与安全性”里手动放行，或在 Finder 中按住 Control 点按应用后选择“打开”。

没有 Apple Developer ID 时，项目目标是：

- `.app` 包结构和 `Info.plist` 正确，让 Finder、LaunchServices 和 Spotlight 能把它识别为应用。
- 应用包通过 `codesign --verify --deep --strict`，避免出现“已损坏”或签名内部错误。
- GitHub 下载后保留 macOS 的 Gatekeeper 提示，让用户通过系统设置手动放行。

不能承诺的是：

- 不能让 macOS 显示“已验证开发者”。
- 不能跳过首次打开的 Gatekeeper 警告。
- 不能保证所有 macOS 版本都只出现同一个按钮文案；常见文案包括“仍要打开”“仍然打开”或 Finder 右键打开确认。

面向用户的 macOS 安装说明应写成：

1. 下载 `DevDock_版本_darwin_架构.dmg`。
2. 打开 DMG，把 `DevDock.app` 拖到“应用程序”。
3. 从“应用程序”里首次打开 `DevDock.app`。
4. 如果提示无法验证开发者，进入“系统设置 > 隐私与安全性”，在页面底部点击 `DevDock` 对应的“仍要打开/仍然打开”。
5. 如果系统设置里没有按钮，在 Finder 的“应用程序”中按住 Control 点按 `DevDock.app`，选择“打开”，再确认打开。

发布前在 macOS 上执行：

```bash
npm run build-dmg
npm run check:macos-release
```

CI 在 macOS Release 构建后也会运行 `scripts/check-macos-unnotarized-release.sh`。当没有配置 Apple 证书时，脚本会要求产物是 ad-hoc 签名，并检查应用包、签名和 DMG 完整性。

如果要让下载的 DMG 正常双击安装并减少“已损坏”“无法验证开发者”等提示，需要使用 Apple Developer 的 `Developer ID Application` 证书并完成 notarization。CI 会在配置证书后自动导入临时 keychain，并查找 `Developer ID Application` 签名身份。

需要配置以下 Secrets：

```text
APPLE_CERTIFICATE
APPLE_CERTIFICATE_PASSWORD
APPLE_ID
APPLE_PASSWORD
APPLE_TEAM_ID
KEYCHAIN_PASSWORD
```

其中 `APPLE_CERTIFICATE` 是导出的 `.p12` 证书 base64 内容：

```bash
openssl base64 -A -in /path/to/certificate.p12 -out certificate-base64.txt
gh secret set APPLE_CERTIFICATE < certificate-base64.txt
```

## 发布版本

发布版本以 tag 为准，tag 可以带 `v` 前缀。CI 会在临时 checkout 的工作区里执行 `npm run version:set -- <tag 版本>`，把 `package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 和 `src-tauri/tauri.conf.json` 的版本临时改成 tag 对应版本再构建；这些版本号变更不需要提前提交到仓库。

示例：

```bash
git tag v0.1.0
git push origin v0.1.0
```

workflow 会校验：

- tag 指向的提交必须在 `origin/master` 上。
- tag 版本必须是 `0.1.6` 这类 SemVer 格式；可以带 `v` 前缀。
- `TAURI_SIGNING_PRIVATE_KEY` 必须存在。

通过校验后，workflow 会按 tag 版本构建 macOS、Windows 和 Linux 产物，创建或更新对应 tag 的 GitHub Release，并上传安装包、updater 包、签名文件和 `latest.json`。

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
