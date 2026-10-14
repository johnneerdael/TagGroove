//! Debug-only end-to-end hook. When a debug build starts with
//! `OPENTAG_E2E_SCRIPT=<file>`, the script is evaluated in the main webview
//! after each page load so native workflows can be driven against the real
//! backend and real files. Release builds never read the variable.

fn enabled() -> bool {
    cfg!(debug_assertions) && std::env::var_os("OPENTAG_E2E_SCRIPT").is_some()
}

pub fn install(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    if !enabled() {
        return builder;
    }
    builder.on_page_load(|webview, payload| {
        if payload.event() != tauri::webview::PageLoadEvent::Finished {
            return;
        }
        let Some(path) = std::env::var_os("OPENTAG_E2E_SCRIPT") else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(script) => {
                if let Err(error) = webview.eval(&script) {
                    eprintln!("[e2e] script could not be evaluated: {error}");
                }
            }
            Err(error) => eprintln!("[e2e] script could not be read: {error}"),
        }
    })
}

/// Prints one line of test output; ignored unless the hook is enabled.
#[tauri::command]
pub fn e2e_report(message: String) {
    if enabled() {
        println!("[e2e] {message}");
    }
}

/// Ends a scripted run; ignored unless the hook is enabled.
#[tauri::command]
pub fn e2e_exit(app: tauri::AppHandle, code: i32) {
    if enabled() {
        app.exit(code);
    }
}
