use tauri::webview::PageLoadEvent;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Dev keeps its own data folder; a debug build on the prod identifier would use real data.
            if cfg!(debug_assertions) && !app.config().identifier.ends_with(".dev") {
                return Err("Debug builds must run with the dev config: npm run app".into());
            }
            Ok(())
        })
        // The window starts hidden; show it once the page is ready.
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let _ = webview.window().show(); // nothing sensible to do if the OS refuses
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running DM's Koldron");
}
