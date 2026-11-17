mod addons;
mod bundles;
mod commands;
mod dice;
mod packs;
mod storage;
mod window_state;

use tauri::webview::PageLoadEvent;
use tauri::{Manager, WindowEvent};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Addon pages, each from its own folder only (commands/addons.rs).
        .register_asynchronous_uri_scheme_protocol(commands::addons::SCHEME, |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            tauri::async_runtime::spawn_blocking(move || responder.respond(commands::addons::serve(&app, &request)));
        })
        .setup(|app| {
            // Dev keeps its own data folder; a debug build on the prod identifier would use real data.
            if cfg!(debug_assertions) && !app.config().identifier.ends_with(".dev") {
                return Err("Debug builds must run with the dev config: npm run app".into());
            }
            // So there's a place to put addons from the first launch on (addons/addons.md). Missing just means no addons.
            if let Ok(dir) = storage::data_dir(app.handle()) {
                let _ = std::fs::create_dir_all(dir.join("addons"));
            }
            commands::search::warm(app.handle().clone());
            if let Some(main) = app.get_webview_window("main") {
                window_state::restore(&main.as_ref().window());
            }
            Ok(())
        })
        // The window starts hidden; show it once the page is ready, already at its saved size and place.
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = webview.window().show(); // nothing sensible to do if the OS refuses
            }
        })
        .on_window_event(|window, event| match event {
            WindowEvent::Resized(_) | WindowEvent::Moved(_) => window_state::track(window),
            WindowEvent::CloseRequested { .. } => window_state::save(window),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::about::license_texts,
            commands::addons::list_addons,
            commands::addons::addon_packs,
            commands::addons::addon_install_pack,
            commands::addons::addon_remove_pack,
            commands::addons::addon_fetch,
            commands::addons::addon_pick_file,
            commands::addons::addon_pick_folder,
            commands::addons::addon_storage_get,
            commands::addons::addon_storage_set,
            commands::addons::addon_characters,
            commands::addons::addon_save_character,
            commands::addons::addon_set_character_picture,
            commands::addons::addon_folders,
            commands::addons::addon_add_folder,
            commands::addons::addon_remove_folder,
            commands::addons::addon_scan_folder,
            commands::addons::addon_read_file,
            commands::addons::addon_hash_file,
            commands::addons::addon_download_file,
            commands::addons::addon_secret_get,
            commands::addons::addon_secret_set,
            commands::addons::addon_secret_delete,
            commands::addons::addon_save_file,
            commands::addons::addon_put_file,
            commands::addons::addon_get_file,
            commands::addons::addon_delete_file,
            commands::addons::addon_list_files,
            commands::addons::addon_reveal_file,
            commands::addons::addon_set_library_visible,
            commands::campaigns::list_campaigns,
            commands::campaigns::create_campaign,
            commands::campaigns::open_campaign,
            commands::campaigns::rename_campaign,
            commands::campaigns::set_party_name,
            commands::campaigns::archive_campaign,
            commands::campaigns::restore_campaign,
            commands::campaigns::delete_campaign,
            commands::campaigns::set_campaign_picture,
            commands::campaigns::remove_campaign_picture,
            commands::catalog::catalog_rows,
            commands::catalog::catalog_entry,
            commands::characters::list_characters,
            commands::characters::save_character,
            commands::characters::delete_character,
            commands::characters::set_character_picture,
            commands::dice::roll_dice,
            commands::encounters::list_encounters,
            commands::encounters::get_encounter,
            commands::encounters::save_encounter,
            commands::encounters::set_encounter_map,
            commands::encounters::encounter_budgets,
            commands::encounters::get_run,
            commands::encounters::save_run,
            commands::encounters::end_run,
            commands::encounters::delete_encounter,
            commands::home::get_home_layout,
            commands::home::set_home_layout,
            commands::sessions::list_sessions,
            commands::sessions::get_session,
            commands::sessions::save_session,
            commands::sessions::delete_session,
            commands::world::get_world,
            commands::world::save_world,
            commands::world::add_location_image,
            commands::world::set_location_map,
            commands::world::set_npc_portrait,
            commands::images::list_pack_images,
            commands::images::entry_image,
            commands::images::download_images,
            commands::images::cancel_image_download,
            commands::images::delete_images,
            commands::packs::list_packs,
            commands::rules::rules_contents,
            commands::rules::rules_chapter,
            commands::rules::rules_section,
            commands::search::search_all,
            commands::settings::get_settings,
            commands::settings::set_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DM's Koldron");
}
