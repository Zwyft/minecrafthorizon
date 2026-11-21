#!/usr/bin/env python3
"""Build the Android APK: cross-compile the game for one ABI, then package it with Gradle.

The game needs the Android NDK, an SDK platform, a JDK and Gradle 8; this script only drives
them. Prepared Minecraft resources are not packaged (Horizon ships none of Mojang's files): copy
the player's own `.local/horizon-client/<version>` folder to the phone, as docs/android.md says.

    python3 tools/android_build.py --abi arm64-v8a
    python3 tools/android_build.py --abi arm64-v8a --release

Outputs `.local/android/jniLibs/<abi>/libhorizon_android.so` and
`.local/android/horizon-android-<abi>.apk`.
"""
import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
API = 28
ABIS = {
    "arm64-v8a": ("aarch64-linux-android", "aarch64-linux-android"),
    "armeabi-v7a": ("armv7-linux-androideabi", "armv7a-linux-androideabi"),
    "x86_64": ("x86_64-linux-android", "x86_64-linux-android"),
}


def fail(message):
    print(f"android_build: {message}", file=sys.stderr)
    raise SystemExit(1)


def find_ndk(argument):
    candidates = [
        argument,
        os.environ.get("ANDROID_NDK_HOME"),
        os.environ.get("ANDROID_NDK_ROOT"),
    ]
    sdk = os.environ.get("ANDROID_HOME")
    if sdk:
        candidates.append(str(Path(sdk) / "ndk"))
    for candidate in candidates:
        if not candidate:
            continue
        path = Path(candidate)
        if (path / "toolchains" / "llvm" / "prebuilt").is_dir():
            return path
        if path.is_dir():
            # An SDK's ndk/ folder holding versioned installs.
            versions = sorted((p for p in path.iterdir() if p.is_dir()), reverse=True)
            for version in versions:
                if (version / "toolchains" / "llvm" / "prebuilt").is_dir():
                    return version
    fail("no Android NDK found: pass --ndk, or set ANDROID_NDK_HOME (NDK 26 or newer)")


def find_host_tag(ndk):
    prebuilt = ndk / "toolchains" / "llvm" / "prebuilt"
    for tag in ("linux-x86_64", "darwin-x86_64", "windows-x86_64"):
        if (prebuilt / tag).is_dir():
            return tag
    fail(f"no host toolchain under {prebuilt}")


def run(command, env=None):
    print("+", " ".join(str(part) for part in command))
    result = subprocess.run(command, cwd=ROOT, env=env)
    if result.returncode != 0:
        raise SystemExit(result.returncode)


def target_installed(target):
    try:
        installed = subprocess.run(
            ["rustup", "target", "list", "--installed"],
            capture_output=True,
            text=True,
            check=False,
        )
    except FileNotFoundError:
        return False
    return target in installed.stdout.split()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--abi", default="arm64-v8a", choices=sorted(ABIS))
    parser.add_argument("--ndk", help="the NDK folder (default: ANDROID_NDK_HOME)")
    parser.add_argument("--sdk", help="the Android SDK folder (default: ANDROID_HOME)")
    parser.add_argument("--gradle", default="gradle", help="the Gradle 8 executable")
    parser.add_argument("--release", action="store_true", help="package a release APK")
    args = parser.parse_args()

    target, clang_prefix = ABIS[args.abi]
    if not target_installed(target):
        fail(f"the Rust target is missing: run `rustup target add {target}`")
    ndk = find_ndk(args.ndk)
    toolchain = ndk / "toolchains" / "llvm" / "prebuilt" / find_host_tag(ndk) / "bin"
    clang = toolchain / f"{clang_prefix}{API}-clang"
    if not clang.is_file():
        fail(f"no compiler for {args.abi} at {clang}")

    environment = dict(os.environ)
    environment["CC_" + target] = str(clang)
    environment["CXX_" + target] = str(toolchain / f"{clang_prefix}{API}-clang++")
    environment["AR_" + target] = str(toolchain / "llvm-ar")
    environment["RANLIB_" + target] = str(toolchain / "llvm-ranlib")
    environment["CFLAGS_" + target] = f"--target={target}{API}"
    environment["CARGO_TARGET_" + target.upper().replace("-", "_") + "_LINKER"] = str(clang)
    if args.sdk:
        environment["ANDROID_HOME"] = args.sdk
    environment["ANDROID_NDK_HOME"] = str(ndk)

    run(
        [
            "cargo",
            "build",
            "--release",
            "--target",
            target,
            "-p",
            "horizon-android",
        ],
        env=environment,
    )

    library = ROOT / "target" / target / "release" / "libhorizon_android.so"
    if not library.is_file():
        fail(f"the build produced no {library.name}")
    jni = ROOT / ".local" / "android" / "jniLibs" / args.abi
    jni.mkdir(parents=True, exist_ok=True)
    shutil.copy2(library, jni / "libhorizon_android.so")

    variant = "release" if args.release else "debug"
    run(
        [
            args.gradle,
            "--no-daemon",
            "-p",
            "packaging/android",
            "-Pabi=" + args.abi,
            "assemble" + variant.title(),
        ],
        env=environment,
    )
    built = ROOT / ".local" / "android" / "gradle-build" / "outputs" / "apk" / variant
    apks = sorted(built.glob("*.apk")) if built.is_dir() else []
    if not apks:
        fail(f"Gradle produced no APK under {built}")
    apk = ROOT / ".local" / "android" / f"horizon-android-{args.abi}.apk"
    shutil.copy2(apks[0], apk)
    print(f"\nAPK: {apk}")
    print(f"Install with: adb install -r {apk}")


if __name__ == "__main__":
    main()
