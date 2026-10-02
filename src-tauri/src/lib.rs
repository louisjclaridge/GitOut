mod auth;
mod commands;
mod error;
mod git;
mod store;

use commands::AppState;
use std::sync::atomic::AtomicU64;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder
            // A second launch focuses the existing window instead of opening another.
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.unminimize();
                    let _ = w.set_focus();
                }
            }))
            .plugin(tauri_plugin_updater::Builder::new().build());
    }

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            app.manage(AppState {
                store: store::Store::load(dir.clone()),
                tokens: auth::TokenStore::new(&dir),
                auth_generation: AtomicU64::new(0),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings_get,
            commands::settings_update,
            commands::git_version,
            commands::repo_open,
            commands::repo_init,
            commands::repo_clone,
            commands::remotes,
            commands::status,
            commands::log,
            commands::refs,
            commands::stashes,
            commands::commit_info,
            commands::diff,
            commands::stage,
            commands::unstage,
            commands::discard,
            commands::apply_patch,
            commands::commit,
            commands::head_message,
            commands::checkout,
            commands::branch_create,
            commands::branch_rename,
            commands::branch_delete,
            commands::remote_branch_delete,
            commands::merge,
            commands::rebase,
            commands::cherry_pick,
            commands::revert,
            commands::reset,
            commands::operation_abort,
            commands::operation_continue,
            commands::tag_create,
            commands::tag_delete,
            commands::push_tag,
            commands::stash_save,
            commands::stash_action,
            commands::fetch,
            commands::pull,
            commands::push,
            commands::auth_device_start,
            commands::auth_device_poll,
            commands::auth_cancel,
            commands::auth_token,
            commands::auth_sign_out,
            commands::auth_list_repos,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GitOut");
}
