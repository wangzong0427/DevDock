#!/usr/bin/env bash
set -euo pipefail

expect_adhoc=0

for arg in "$@"; do
  case "$arg" in
    --expect-adhoc)
      expect_adhoc=1
      ;;
    *)
      echo "未知参数：$arg" >&2
      exit 2
      ;;
  esac
done

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "此脚本只能在 macOS 上运行。"
  exit 0
fi

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="$root_dir/src-tauri/target"

if [[ ! -d "$target_dir" ]]; then
  echo "未找到构建目录：$target_dir" >&2
  exit 1
fi

plist_value() {
  local plist="$1"
  local key="$2"
  /usr/libexec/PlistBuddy -c "Print :$key" "$plist"
}

verify_app() {
  local app="$1"
  local run_spctl="${2:-1}"
  local plist="$app/Contents/Info.plist"
  local executable_dir="$app/Contents/MacOS"
  local bundle_type
  local bundle_id
  local bundle_name
  local sign_info
  local spctl_output
  local spctl_status

  plist="$app/Contents/Info.plist"
  executable_dir="$app/Contents/MacOS"

  echo "校验应用包：$app"

  if [[ ! -f "$plist" ]]; then
    echo "缺少 Info.plist：$plist" >&2
    exit 1
  fi

  if [[ ! -d "$executable_dir" ]]; then
    echo "缺少 Contents/MacOS 目录：$executable_dir" >&2
    exit 1
  fi

  bundle_type="$(plist_value "$plist" CFBundlePackageType)"
  bundle_id="$(plist_value "$plist" CFBundleIdentifier)"
  bundle_name="$(plist_value "$plist" CFBundleName)"

  if [[ "$bundle_type" != "APPL" ]]; then
    echo "CFBundlePackageType 必须是 APPL，实际为：$bundle_type" >&2
    exit 1
  fi

  if [[ -z "$bundle_id" || -z "$bundle_name" ]]; then
    echo "CFBundleIdentifier 和 CFBundleName 不能为空。" >&2
    exit 1
  fi

  codesign --verify --deep --strict --verbose=2 "$app"

  sign_info="$(codesign -dv --verbose=4 "$app" 2>&1)"
  echo "$sign_info" | grep -E "^(Identifier|Signature|TeamIdentifier)=" || true

  if [[ "$expect_adhoc" -eq 1 ]] && ! grep -q "Signature=adhoc" <<<"$sign_info"; then
    echo "当前发布预期为 ad-hoc 签名，但产物不是 ad-hoc 签名。" >&2
    exit 1
  fi

  if [[ "$run_spctl" -eq 1 ]]; then
    set +e
    spctl_output="$(spctl --assess --type execute -vv "$app" 2>&1)"
    spctl_status=$?
    set -e

    echo "$spctl_output"

    if [[ "$expect_adhoc" -eq 1 && "$spctl_status" -eq 0 ]]; then
      echo "警告：ad-hoc 产物被 spctl 接受，这不符合常见 Gatekeeper 行为，请确认 CI 环境。"
    elif [[ "$expect_adhoc" -eq 1 ]]; then
      echo "spctl 未接受 ad-hoc/未公证应用，这是预期结果；用户首次打开需要手动放行。"
    elif [[ "$spctl_status" -ne 0 ]]; then
      echo "警告：spctl 未接受此应用，请确认是否缺少 notarization 或签名配置。"
    fi
  fi
}

found_apps=0
while IFS= read -r app; do
  found_apps=1
  verify_app "$app"
done < <(find "$target_dir" -path "*/bundle/macos/*.app" -type d | sort)

found_dmgs=0
while IFS= read -r dmg; do
  found_dmgs=1
  echo "校验 DMG：$dmg"
  hdiutil verify "$dmg"

  mount_dir="$(mktemp -d "${TMPDIR:-/tmp}/devdock-dmg.XXXXXX")"
  attached=0

  if hdiutil attach -readonly -nobrowse -noautoopen -mountpoint "$mount_dir" "$dmg"; then
    attached=1
    found_dmg_apps=0
    while IFS= read -r dmg_app; do
      found_dmg_apps=1
      verify_app "$dmg_app" 0
    done < <(find "$mount_dir" -maxdepth 2 -name "*.app" -type d | sort)

    if [[ "$found_dmg_apps" -eq 0 ]]; then
      echo "DMG 内未找到 .app：$dmg" >&2
      hdiutil detach "$mount_dir"
      rmdir "$mount_dir"
      exit 1
    fi

    hdiutil detach "$mount_dir"
    attached=0
  else
    echo "无法挂载 DMG：$dmg" >&2
    rmdir "$mount_dir"
    exit 1
  fi

  if [[ "$attached" -eq 1 ]]; then
    hdiutil detach "$mount_dir" || true
  fi
  rmdir "$mount_dir" || true
done < <(find "$target_dir" -path "*/bundle/dmg/*.dmg" -type f | sort)

if [[ "$found_apps" -eq 0 && "$found_dmgs" -eq 0 ]]; then
  echo "未找到 macOS .app 或 DMG 产物，请先运行 npm run tauri build。"
  exit 1
fi

echo "macOS 产物校验完成。"
