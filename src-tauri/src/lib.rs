mod bundles;
mod commands;
mod packs;
mod storage;

use tauri::webview::PageLoadEvent;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Dev keeps its own data folder; a debug build on the prod identifier would use real data.
            if cfg!(debug_assertions) && !app.config().identifier.ends_with(".dev") {
                return Err("Debug builds must run with the dev config: npm run app".into());
            }
            commands::search::warm(app.handle().clone());
            Ok(())
        })
        // The window starts hidden; show it once the page is ready.
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = webview.window().show(); // nothing sensible to do if the OS refuses
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::about::license_texts,
            commands::campaigns::list_campaigns,
            commands::campaigns::create_campaign,
            commands::campaigns::open_campaign,
            commands::campaigns::rename_campaign,
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
