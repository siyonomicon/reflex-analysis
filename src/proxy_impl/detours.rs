/// Detours module for selectively intercepting functions from the original DLL
///
/// This module demonstrates how to:
/// 1. Hook specific functions by offset (for internal functions)
/// 2. Hook exported functions by name
/// 3. Replace functionality while optionally calling the original
/// 4. Implement custom behavior

use crate::proxy;
use winapi::shared::minwindef::{BOOL, DWORD, LPVOID};
use winapi::um::winnt::{HANDLE, LPCSTR, LPCWSTR, LPWSTR};

/// Example: Hook an internal function by offset
///
/// To find the offset:
/// 1. Use radare2: `r2 -q -c "aaa; afl" reflex_original.dll`
/// 2. Find the function address (e.g., 0x180001234)
/// 3. Calculate offset from base (0x180000000): 0x1234
///
/// # Safety
/// This is extremely unsafe and depends on exact binary layout.
/// Offsets will change if the DLL is recompiled or updated.
pub unsafe fn hook_internal_function_example() {
    // Example: Hook a function at offset 0x1234 from DLL base
    const FUNCTION_OFFSET: usize = 0x1234;

    type InternalFunctionType = unsafe extern "system" fn(DWORD, LPVOID) -> BOOL;

    if let Some(original_fn) = proxy::resolve_internal_function::<InternalFunctionType>(FUNCTION_OFFSET) {
        log::info!("[detours] Successfully resolved internal function at offset 0x{:x}", FUNCTION_OFFSET);

        // You can now call the original function
        // let result = original_fn(param1, param2);

        // Or store it for later use in your hook
        // ORIGINAL_INTERNAL_FN = Some(original_fn);
    } else {
        log::error!("[detours] Failed to resolve internal function at offset 0x{:x}", FUNCTION_OFFSET);
    }
}

/// Example: Hook an exported function by name
///
/// This is safer than offset-based hooking because it uses the export table.
pub unsafe fn hook_exported_function_example() {
    type DllMainType = unsafe extern "system" fn(LPVOID, DWORD, LPVOID) -> BOOL;

    if let Some(original_dllmain) = proxy::get_original_export::<DllMainType>("DllMain") {
        log::info!("[detours] Successfully resolved exported DllMain");
        // Store for later use
    } else {
        log::warn!("[detours] DllMain not found in exports (this is normal)");
    }
}

// ============================================================================
// Example Hook Implementations
// ============================================================================

/// Example: Hook for DeleteFileW
///
/// This demonstrates how to intercept a Windows API call that the original
/// DLL might be hooking, and add your own custom behavior.
pub unsafe extern "system" fn hooked_delete_file_w(file_name: LPCWSTR) -> BOOL {
    // Convert wide string to Rust string for logging
    let path = wstr_to_string(file_name);

    log::info!("[detours] DeleteFileW intercepted: {}", path);

    // Add custom logic here
    if path.contains("important_file") {
        log::warn!("[detours] Blocking deletion of important file: {}", path);
        return 0; // FALSE - block deletion
    }

    // Call the original function from reflex_original.dll
    // You would need to resolve this first and store it
    // For now, just return success
    1 // TRUE
}

/// Example: Hook for GetUserNameW
///
/// This shows how to spoof return values
pub unsafe extern "system" fn hooked_get_user_name_w(buffer: LPWSTR, size: *mut DWORD) -> BOOL {
    log::info!("[detours] GetUserNameW intercepted");

    // Return a custom username
    let custom_username = "CustomUser";
    let username_wide: Vec<u16> = custom_username
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    if (*size as usize) < username_wide.len() {
        *size = username_wide.len() as DWORD;
        return 0; // FALSE - buffer too small
    }

    std::ptr::copy_nonoverlapping(username_wide.as_ptr(), buffer, username_wide.len());
    *size = username_wide.len() as DWORD;

    1 // TRUE
}

/// Example: Hook for registry operations
///
/// This demonstrates intercepting registry queries
pub unsafe extern "system" fn hooked_reg_query_value_ex_w(
    key: HANDLE,
    value_name: LPCWSTR,
    reserved: *mut DWORD,
    type_: *mut DWORD,
    data: *mut u8,
    data_size: *mut DWORD,
) -> i32 {
    let name = wstr_to_string(value_name);
    log::info!("[detours] RegQueryValueExW intercepted: {}", name);

    // Spoof specific registry values
    if name == "HwProfileGuid" {
        log::info!("[detours] Spoofing HwProfileGuid");
        // Return custom GUID
        let custom_guid = "{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}";
        let guid_wide: Vec<u16> = custom_guid
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        if !data.is_null() && (*data_size as usize) >= guid_wide.len() * 2 {
            std::ptr::copy_nonoverlapping(
                guid_wide.as_ptr() as *const u8,
                data,
                guid_wide.len() * 2,
            );
        }

        return 0; // ERROR_SUCCESS
    }

    // For other values, call original or return error
    0 // ERROR_SUCCESS
}

// ============================================================================
// Function Pointer Storage
// ============================================================================

/// Storage for original function pointers
///
/// These would be initialized during DLL_PROCESS_ATTACH by resolving
/// functions from the original DLL.
pub struct OriginalFunctions {
    // Windows API hooks (if the original DLL hooks them)
    pub delete_file_w: Option<unsafe extern "system" fn(LPCWSTR) -> BOOL>,
    pub get_user_name_w: Option<unsafe extern "system" fn(LPWSTR, *mut DWORD) -> BOOL>,
    pub reg_query_value_ex_w: Option<unsafe extern "system" fn(HANDLE, LPCWSTR, *mut DWORD, *mut DWORD, *mut u8, *mut DWORD) -> i32>,

    // Internal reflex.dll functions (by offset)
    pub internal_init_fn: Option<unsafe extern "system" fn() -> BOOL>,
    pub internal_cleanup_fn: Option<unsafe extern "system" fn() -> BOOL>,
}

impl OriginalFunctions {
    pub const fn new() -> Self {
        Self {
            delete_file_w: None,
            get_user_name_w: None,
            reg_query_value_ex_w: None,
            internal_init_fn: None,
            internal_cleanup_fn: None,
        }
    }
}

static mut ORIGINAL_FUNCTIONS: OriginalFunctions = OriginalFunctions::new();

/// Initialize detours by resolving original functions
///
/// Call this during DLL_PROCESS_ATTACH after the proxy is initialized
pub unsafe fn initialize_detours() -> Result<(), String> {
    log::info!("[detours] Initializing detours...");

    // Example: Resolve internal functions by offset
    // These offsets would come from reverse engineering with radare2

    // Example offset for an initialization function
    const INIT_FN_OFFSET: usize = 0x1000; // Replace with actual offset
    ORIGINAL_FUNCTIONS.internal_init_fn =
        proxy::resolve_internal_function(INIT_FN_OFFSET);

    // Example offset for a cleanup function
    const CLEANUP_FN_OFFSET: usize = 0x2000; // Replace with actual offset
    ORIGINAL_FUNCTIONS.internal_cleanup_fn =
        proxy::resolve_internal_function(CLEANUP_FN_OFFSET);

    log::info!("[detours] Detours initialized successfully");
    Ok(())
}

/// Call an original internal function if it was resolved
pub unsafe fn call_original_init() -> Result<(), String> {
    if let Some(init_fn) = ORIGINAL_FUNCTIONS.internal_init_fn {
        log::debug!("[detours] Calling original init function");
        let result = init_fn();
        if result == 0 {
            return Err("Original init function failed".to_string());
        }
        Ok(())
    } else {
        Err("Original init function not resolved".to_string())
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Convert a wide string pointer to a Rust String
unsafe fn wstr_to_string(ptr: LPCWSTR) -> String {
    if ptr.is_null() {
        return String::new();
    }

    let mut len = 0;
    while *ptr.offset(len) != 0 {
        len += 1;
    }

    let slice = std::slice::from_raw_parts(ptr, len as usize);
    String::from_utf16_lossy(slice)
}

/// Convert an ANSI string pointer to a Rust String
unsafe fn str_to_string(ptr: LPCSTR) -> String {
    if ptr.is_null() {
        return String::new();
    }

    std::ffi::CStr::from_ptr(ptr)
        .to_string_lossy()
        .into_owned()
}
