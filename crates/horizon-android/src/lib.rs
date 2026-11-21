//! Android's native entry point: the activity winit's Android backend starts (`android_main`),
//! the folders the game keeps its world and resources in, and the game itself.
//!
//! A phone has no command line, so the folders are passed as the arguments a desktop start would
//! give (`--game-dir`, `--resources`). The game is the official one (`horizon-official`'s hooks),
//! exactly as the desktop executable runs it.
#![cfg(target_os = "android")]

use std::path::{Path, PathBuf};
use winit::platform::android::activity::AndroidApp;

/// Identifies this executable as the game, as the desktop executables do.
static GAME_IDENTITY: [u8; horizon_mod_package::game_identity::MARK.len()] =
    horizon_mod_package::game_identity::MARK;

/// Where the player's own files go: worlds, options, packs. Android gives the app this folder,
/// writable without a storage permission, and a file manager can reach it under
/// `Android/data/<package>/files`.
fn files_dir(app: &AndroidApp, home: &Path) -> PathBuf {
    app.external_data_path()
        .unwrap_or_else(|| home.to_path_buf())
}

#[unsafe(no_mangle)]
extern "C" fn android_main(app: AndroidApp) {
    horizon_guest_stack::prepare_main();
    horizon_mod_host::exit_if_compile_child();
    std::hint::black_box(&GAME_IDENTITY);
    let Some(home) = app.internal_data_path() else {
        return;
    };
    let files = files_dir(&app, &home);
    let game_dir = files.join("game");
    if std::fs::create_dir_all(&game_dir).is_err() {
        return;
    }
    // Prepared resources (horizon-prepare on a computer) copied beside the game folder:
    // <files>/horizon-client/<version>/, which is what --resources means on the desktop.
    let resources = files
        .join("horizon-client")
        .join(horizon_core::VANILLA_VERSION);
    let arguments = vec![
        "--game-dir".to_owned(),
        game_dir.display().to_string(),
        "--resources".to_owned(),
        resources.display().to_string(),
    ];
    // The loop the game opens its window on carries the activity's app.
    horizon_client::android::install(app.clone());
    horizon_official::register();
    let startup = horizon_client::entry::prepare_with(arguments);
    horizon_mod_host::main_with(Vec::new(), &[], &[], startup);
    // The activity stays cached after the game ends; winit allows one loop per process, so the
    // next launch starts a fresh one.
    std::process::exit(0);
}
