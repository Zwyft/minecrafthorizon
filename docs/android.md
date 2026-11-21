# Horizon on Android

An APK that runs the game natively on a phone: winit's Android backend for the window, wgpu on
Vulkan, and the on-screen touch controls the client already carries. It is a development build,
not a release: no official APK is published, and it has not been tested on a device by the
project (see [Status](#status)).

## What it does

- One activity (`android.app.NativeActivity`, no Java code) starts `android_main`
  (`crates/horizon-android`), which runs the same game as the desktop executable, with
  `horizon-official`'s parts registered.
- The player's files live in the app's external files folder, `Android/data/dev.horizon.android/files`:
  `game/` holds worlds, `options.txt` and packs; `horizon-client/<version>/` holds prepared
  resources.
- In play, fingers become the controller the game already reads (`game::interface::android_touch`):
  the joystick moves, the buttons jump, sneak, sprint, attack, use, open the inventory, pause and
  so on, and a drag on the world turns the view. The controls are drawn over the HUD.
- On every other screen a tap is a left click where it landed, so the title screen, options and
  inventories work without a mouse. The chat, F3 and F5 buttons press those keys; the settings
  button opens the pause screen.
- The default scheme aims at the crosshair, so attack and use act where the crosshair points.
  Holding one of those buttons mines or uses the held item; a short tap does not aim at the
  tapped block yet.

Not there yet: tap-to-interact aiming for the tap and D-pad schemes, gamepads (this build leaves
out the `controllers` feature, which needs gilrs), sound (`audio` is off in the Android build),
the Android back button (it finishes the activity; pause first), a launcher, and on-device
resource preparation (resources are prepared on a computer and copied over).

## Build

Needs Rust 1.95 with the Android target, the Android SDK (platform 35, build tools), NDK 26 or
newer, a JDK 17+ and Gradle 8.

```sh
rustup target add aarch64-linux-android
export ANDROID_HOME=$HOME/Android/Sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.0.12077973
python3 tools/android_build.py --abi arm64-v8a            # debug APK
python3 tools/android_build.py --abi arm64-v8a --release  # release APK
```

The script finds the NDK (or takes `--ndk`), points Cargo's linker and C compiler at it, builds
`horizon-android`, copies `libhorizon_android.so` to `.local/android/jniLibs/<abi>/` and runs
Gradle on `packaging/android`. The APK is written to `.local/android/horizon-android-<abi>.apk`.
Only `arm64-v8a`, `armeabi-v7a` and `x86_64` are set up; an emulator needs the matching ABI.

The build leaves out `controllers` and `audio` (`crates/horizon-android/Cargo.toml`): the phone
plays with the touch controls, and neither gilrs nor cpal has an Android backend this build uses.

## Install and play

1. Install the APK: `adb install -r .local/android/horizon-android-arm64-v8a.apk`.
2. Prepare resources on a computer with `horizon-prepare`, or let the launcher do it:
   `cargo run -p horizon-prepare`, which writes `.local/horizon-client/<version>/` from the
   player's own Minecraft install.
3. Copy that folder to the phone:
   `adb push .local/horizon-client/26.3 /sdcard/Android/data/dev.horizon.android/files/horizon-client/`
4. Start Horizon. The loading screen reads those resources and opens the title screen, as on
   the desktop.

The APK contains no Minecraft files: resources are the player's own, copied by them. Without
them the game shows its missing-resources screen.

## Where the code is

| Part | File |
|---|---|
| Activity entry, folders, game start | `crates/horizon-android/src/lib.rs` |
| The activity's app for the event loop | `crates/horizon-client/src/android.rs`, `game/runtime/event_loop.rs` |
| Explicit command line (no process arguments) | `crates/horizon-client/src/entry.rs` (`prepare_with`) |
| Touch → pad, taps on screens, HUD drawing | `crates/horizon-client/src/game/interface/android_touch.rs` |
| Touch controls and their layout | `crates/horizon-client-render/src/touch/` |
| Manifest and Gradle project | `packaging/android/` |
| Build script | `tools/android_build.py` |

## Status

What is checked: the touched Rust files parse and are `rustfmt`-clean, `cargo metadata` accepts
the new crate and feature wiring, and the manifest, the Python build script and the Gradle files
parse. What is not: nothing here was compiled for Android or run on a phone, because the
environment that wrote this has no NDK and no device. The first device build should be expected
to need fixes around the activity lifecycle, pointer capture and performance, which only a
device shows.
