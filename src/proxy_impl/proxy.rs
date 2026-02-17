/// Proxy module for loading and forwarding to original reflex.dll
///
/// This module implements a DLL proxy pattern where:
/// 1. This DLL is named reflex.dll
/// 2. Original reflex.dll is renamed to reflex_original.dll
/// 3. All calls are forwarded to the original DLL
/// 4. Optional hooks can intercept/modify behavior

use std::ffi::CString;
use std::sync::Once;
use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, HMODULE, LPVOID, TRUE, FALSE};
use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryA};
use winapi::um::winnt::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};

static INIT: Once = Once::new();
static mut ORIGINAL_DLL: HMODULE = std::ptr::null_mut();
static mut ORIGINAL_DLLMAIN: Option<DllMainFn> = None;

type DllMainFn = unsafe extern "system" fn(HINSTANCE, DWORD, LPVOID) -> BOOL;

/// Configuration for proxy behavior
pub struct ProxyConfig {
    /// Path to the original DLL (default: "reflex_original.dll")
    pub original_dll_path: &'static str,
    /// Enable logging of proxy operations
    pub enable_logging: bool,
    /// Enable pre-hook (called before forwarding to original)
    pub enable_pre_hook: bool,
    /// Enable post-hook (called after forwarding to original)
    pub enable_post_hook: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            original_dll_path: "reflex_original.dll",
            enable_logging: true,
            enable_pre_hook: false,
            enable_post_hook: false,
        }
    }
}

/// Initialize the proxy by loading the original DLL
pub unsafe fn initialize_proxy(config: &ProxyConfig) -> Result<(), String> {
    let dll_path = CString::new(config.original_dll_path)
        .map_err(|e| format!("Invalid DLL path: {}", e))?;

    // Load the original DLL
    let handle = LoadLibraryA(dll_path.as_ptr());
    if handle.is_null() {
        return Err(format!(
            "Failed to load original DLL: {}",
            config.original_dll_path
        ));
    }

    ORIGINAL_DLL = handle;

    if config.enable_logging {
        log::info!(
            "[reflex-proxy] Loaded original DLL from: {}",
            config.original_dll_path
        );
        log::info!("[reflex-proxy] Original DLL base address: {:p}", handle);
    }

    // Get the address of DllMain from the original DLL
    let dllmain_name = CString::new("DllMain").unwrap();
    let dllmain_addr = GetProcAddress(handle, dllmain_name.as_ptr());

    if dllmain_addr.is_null() {
        return Err("Failed to find DllMain in original DLL".to_string());
    }

    ORIGINAL_DLLMAIN = Some(std::mem::transmute(dllmain_addr));

    if config.enable_logging {
        log::info!("[reflex-proxy] Original DllMain at: {:p}", dllmain_addr);
    }

    Ok(())
}

/// Forward DllMain call to the original DLL
pub unsafe fn forward_dllmain(
    hinst_dll: HINSTANCE,
    fdw_reason: DWORD,
    lpv_reserved: LPVOID,
    config: &ProxyConfig,
) -> BOOL {
    // Pre-hook: called before forwarding to original
    if config.enable_pre_hook {
        if let Some(result) = pre_dllmain_hook(hinst_dll, fdw_reason, lpv_reserved) {
            return result;
        }
    }

    // Forward to original DllMain
    let result = if let Some(original_dllmain) = ORIGINAL_DLLMAIN {
        if config.enable_logging {
            log::debug!(
                "[reflex-proxy] Forwarding DllMain(reason={}) to original",
                fdw_reason
            );
        }
        original_dllmain(hinst_dll, fdw_reason, lpv_reserved)
    } else {
        if config.enable_logging {
            log::error!("[reflex-proxy] Original DllMain not initialized!");
        }
        FALSE
    };

    // Post-hook: called after forwarding to original
    if config.enable_post_hook {
        post_dllmain_hook(hinst_dll, fdw_reason, lpv_reserved, result);
    }

    result
}

/// Pre-hook: called before forwarding to original DllMain
/// Return Some(BOOL) to override the call, None to continue forwarding
fn pre_dllmain_hook(
    _hinst_dll: HINSTANCE,
    fdw_reason: DWORD,
    _lpv_reserved: LPVOID,
) -> Option<BOOL> {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            log::info!("[reflex-proxy] Pre-hook: DLL_PROCESS_ATTACH");
            // Add custom initialization here
            // Return Some(TRUE) to skip original DllMain
            // Return None to continue to original
        }
        DLL_PROCESS_DETACH => {
            log::info!("[reflex-proxy] Pre-hook: DLL_PROCESS_DETACH");
            // Add custom cleanup here
        }
        _ => {}
    }
    None // Continue to original
}

/// Post-hook: called after forwarding to original DllMain
fn post_dllmain_hook(
    _hinst_dll: HINSTANCE,
    fdw_reason: DWORD,
    _lpv_reserved: LPVOID,
    result: BOOL,
) {
    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            log::info!(
                "[reflex-proxy] Post-hook: DLL_PROCESS_ATTACH completed with result={}",
                result
            );
            // Add custom post-initialization here
        }
        DLL_PROCESS_DETACH => {
            log::info!(
                "[reflex-proxy] Post-hook: DLL_PROCESS_DETACH completed with result={}",
                result
            );
            // Add custom post-cleanup here
        }
        _ => {}
    }
}

/// Get the base address of the original loaded DLL
pub unsafe fn get_original_dll_base() -> HMODULE {
    ORIGINAL_DLL
}

/// Resolve an internal function address by offset from the original DLL base
///
/// # Safety
/// This is highly unsafe and depends on the exact binary layout.
/// Use only if you know the exact offset from reverse engineering.
pub unsafe fn resolve_internal_function<F>(offset: usize) -> Option<F> {
    if ORIGINAL_DLL.is_null() {
        return None;
    }

    let base = ORIGINAL_DLL as usize;
    let func_addr = base + offset;

    Some(std::mem::transmute_copy(&func_addr))
}

/// Get an exported function from the original DLL by name
pub unsafe fn get_original_export<F>(name: &str) -> Option<F> {
    if ORIGINAL_DLL.is_null() {
        return None;
    }

    let name_cstr = CString::new(name).ok()?;
    let func_addr = GetProcAddress(ORIGINAL_DLL, name_cstr.as_ptr());

    if func_addr.is_null() {
        return None;
    }

    Some(std::mem::transmute_copy(&func_addr))
}
