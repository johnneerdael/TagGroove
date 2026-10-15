//! Debug-only end-to-end hook. When a debug build starts with
//! `OPENTAG_E2E_SCRIPT=<file>`, the script is evaluated in the main webview
//! after each page load so native workflows can be driven against the real
//! backend and real files. Release builds never read the variable unless they
//! are profiling builds compiled with the off-by-default `e2e` feature.

pub(crate) fn enabled() -> bool {
    cfg!(any(debug_assertions, feature = "e2e")) && std::env::var_os("OPENTAG_E2E_SCRIPT").is_some()
}

/// Scripts write shortcuts with the macOS Command key. Elsewhere the app's
/// primary modifier is Ctrl (it checks `navigator.platform`), so synthetic
/// events created with `metaKey` are translated to `ctrlKey`.
const SHORTCUT_PRELUDE: &str = r#"(() => {
  if (/Mac|iPhone|iPad/.test(navigator.platform) || window.__opentagShortcutPrelude) return;
  window.__opentagShortcutPrelude = true;
  const Native = window.KeyboardEvent;
  window.KeyboardEvent = class extends Native {
    constructor(type, init = {}) {
      super(type, init.metaKey ? { ...init, metaKey: false, ctrlKey: true } : init);
    }
  };
})();"#;

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
        // The scripted window must never pin itself above or take focus from a
        // user's windows; native suites run headless in CI (xvfb).
        match std::fs::read_to_string(&path) {
            Ok(script) => {
                if let Err(error) = webview.eval(format!(
                    "window.OPENTAG_E2E_OFFLINE = {};\n{SHORTCUT_PRELUDE}\n{script}",
                    std::env::var_os("OPENTAG_E2E_OFFLINE").is_some()
                )) {
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
