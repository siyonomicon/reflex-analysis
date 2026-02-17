use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID, TRUE};
use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

mod proxy_impl;

use proxy_impl::proxy;
use proxy_impl::detours;

use once_cell::sync::Lazy;
use std::sync::Mutex;

static INITIALIZED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

/// DllMain - Proxy entry point for reflex.dll
///
/// This is a proxy DLL that forwards all calls to the original reflex.dll
/// (renamed to reflex_original.dll).
///
/// Architecture:
/// 1. Application loads version.dll from its directory (version.dll proxy)
/// 2. version.dll proxy loads the real version.dll from System32
/// 3. version.dll proxy loads reflex.dll (THIS DLL - the proxy)
/// 4. This proxy loads reflex_original.dll (the real implementation)
/// 5. All calls are forwarded to reflex_original.dll
/// 6. Optional hooks can intercept/modify behavior
///
/// Benefits:
/// - No need to reimplement everything from scratch
/// - Original functionality continues to work
/// - Can selectively replace/intercept specific functions
/// - Easy to maintain and debug
#[no_mangle]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    hinst_dll: HINSTANCE,
    fdw_reason: DWORD,
    lpv_reserved: LPVOID,
) -> BOOL {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            // Prevent double initialization
            let mut init = INITIALIZED.lock().unwrap();
            if *init {
                return TRUE;
            }

            // Initialize logging first
            if let Err(e) = init_logging() {
                eprintln!("[reflex-proxy] Failed to initialize logging: {}", e);
                return TRUE;
            }

            log::info!("[reflex-proxy] Proxy DLL initializing...");
            log::info!("[reflex-proxy] This is a proxy that forwards to reflex_original.dll");

            // Configure proxy behavior
            let config = proxy::ProxyConfig {
                original_dll_path: "reflex_original.dll",
                enable_logging: true,
                enable_pre_hook: false,  // Set to true to add custom pre-processing
                enable_post_hook: false, // Set to true to add custom post-processing
            };

            // Initialize the proxy (load original DLL)
            unsafe {
                if let Err(e) = proxy::initialize_proxy(&config) {
                    log::error!("[reflex-proxy] Failed to initialize proxy: {}", e);
                    log::error!("[reflex-proxy] Make sure reflex_original.dll exists!");
                    return TRUE;
                }
            }

            log::info!("[reflex-proxy] Proxy initialized successfully");

            // Optional: Initialize detours to intercept specific functions
            // Uncomment the following lines to enable custom hooks
            // unsafe {
            //     if let Err(e) = detours::initialize_detours() {
            //         log::warn!("[reflex-proxy] Failed to initialize detours: {}", e);
            //     }
            // }

            log::info!("[reflex-proxy] Forwarding DllMain to original...");

            *init = true;

            // Forward the DLL_PROCESS_ATTACH to the original DLL
            unsafe { proxy::forward_dllmain(hinst_dll, fdw_reason, lpv_reserved, &config) }
        }

        DLL_PROCESS_DETACH => {
            log::info!("[reflex-proxy] Proxy detaching, forwarding to original...");

            // Configure proxy for detach
            let config = proxy::ProxyConfig {
                original_dll_path: "reflex_original.dll",
                enable_logging: true,
                enable_pre_hook: false,
                enable_post_hook: false,
            };

            // Forward the DLL_PROCESS_DETACH to the original DLL
            unsafe { proxy::forward_dllmain(hinst_dll, fdw_reason, lpv_reserved, &config) }
        }

        _ => {
            // Forward other reasons to original DLL
            let config = proxy::ProxyConfig::default();
            unsafe { proxy::forward_dllmain(hinst_dll, fdw_reason, lpv_reserved, &config) }
        }
    }
}

fn init_logging() -> Result<(), Box<dyn std::error::Error>> {
    use std::fs::OpenOptions;

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("reflex.log")?;

    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Pipe(Box::new(log_file)))
        .init();

    Ok(())
}
