# Irori on Android (card #225, #226)

The phone build is a board client: it pairs with your other devices (Settings → Devices → Join with a code), syncs your cards and boards over iroh, and lets you read, comment, answer blockers and ask. It runs no agents. The tray, single instance, launch at login and the updater are desktop only (`#[cfg(desktop)]` and `app/src/shell/tray_mobile.rs`).

## Toolchain (Mac)

```sh
brew install openjdk@17
brew install --cask android-commandlinetools
export JAVA_HOME=/opt/homebrew/opt/openjdk@17/libexec/openjdk.jdk/Contents/Home
export ANDROID_HOME=/opt/homebrew/share/android-commandlinetools
yes | sdkmanager --sdk_root=$ANDROID_HOME --licenses
sdkmanager --sdk_root=$ANDROID_HOME "platform-tools" "platforms;android-35" "build-tools;35.0.0" "ndk;27.2.12479018"
export NDK_HOME=$ANDROID_HOME/ndk/27.2.12479018
rustup target add aarch64-linux-android   # in the repo, so the pinned toolchain gets it
```

## Build and install

```sh
(cd frontend && bun run build)
cd app
cargo tauri android init --ci            # once; writes app/gen/android (git-ignored)
CARGO_PROFILE_DEV_OPT_LEVEL=3 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo tauri android build --debug --target aarch64 --apk --ci
$ANDROID_HOME/platform-tools/adb install -r gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

On the phone: Settings → About phone → tap Build number 7 times, then Developer options → USB debugging on; plug in and allow the Mac. Use the optimised build above: an unoptimised one takes about 45 s to open a fully synced database (21 s optimised, 2026-11-01).

## Notes

- Data lives in the app's private files dir (`/data/data/com.stephenshorton.irori/files/irori`); `mobile_main` sets `IRORI_HOME`, `HOME` and `CLAUDE_CONFIG_DIR` there before the shell starts.
- The app library is linked with 16 KB pages (`.cargo/config.toml`); without it Android 15+ phones warn that the app isn't 16 KB compatible.
- Logs: `adb shell run-as com.stephenshorton.irori tail files/irori/irori.log`.
