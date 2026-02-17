# Reflex Proxy - Rust Implementation

This directory contains a Rust implementation of a **DLL proxy** for reflex.dll.

## What is This?

Instead of reimplementing reflex.dll from scratch, this proxy:
1. Loads the original reflex.dll (renamed to `reflex_original.dll`)
2. Forwards all calls to it
3. Optionally intercepts specific functions for custom behavior

## Quick Start

### Build

```bash
cd reflex_proxy
cargo build --release
```

Output: `target/release/reflex.dll`

### Deploy

```bash
# In your application directory:
# 1. Rename original
ren reflex.dll reflex_original.dll

# 2. Copy proxy
copy reflex_proxy\target\release\reflex.dll .
```

### Verify

Check `reflex.log` for output:
```
[reflex-proxy] Proxy DLL initializing...
[reflex-proxy] Loaded original DLL from: reflex_original.dll
[reflex-proxy] Forwarding DllMain to original...
```

## Project Structure

```
reflex_proxy/
├── Cargo.toml              # Project configuration
├── Cargo.lock              # Dependency lock file
├── build.rs                # Build script
├── src/
│   ├── lib.rs              # DllMain entry point
│   └── proxy_impl/
│       ├── mod.rs          # Module exports
│       ├── proxy.rs        # DLL loading & forwarding
│       └── detours.rs      # Function interception examples
└── target/                 # Build output
    └── release/
        └── reflex.dll      # Built proxy DLL
```

## Key Files

### [src/lib.rs](src/lib.rs)
- Exports `DllMain` function
- Initializes logging
- Loads and forwards to original DLL

### [src/proxy_impl/proxy.rs](src/proxy_impl/proxy.rs)
- Loads `reflex_original.dll`
- Resolves function addresses
- Forwards DllMain calls
- Provides pre/post hooks

### [src/proxy_impl/detours.rs](src/proxy_impl/detours.rs)
- Examples of function interception
- Shows how to hook by offset
- Shows how to hook by name
- Demonstrates custom behavior

## Customization

### Enable Pre/Post Hooks

Edit `src/lib.rs`:

```rust
let config = proxy::ProxyConfig {
    original_dll_path: "reflex_original.dll",
    enable_logging: true,
    enable_pre_hook: true,   // ✅ Enable
    enable_post_hook: true,  // ✅ Enable
};
```

### Add Custom Hooks

Edit `src/proxy_impl/proxy.rs`:

```rust
fn pre_dllmain_hook(...) -> Option<BOOL> {
    // Your custom code here
    log::info!("Custom initialization!");
    None // Continue to original
}
```

### Intercept Functions

Edit `src/lib.rs` to enable detours:

```rust
// Uncomment these lines:
unsafe {
    if let Err(e) = detours::initialize_detours() {
        log::warn!("[reflex-proxy] Failed to initialize detours: {}", e);
    }
}
```

Then implement hooks in `src/proxy_impl/detours.rs`.

## Finding Function Offsets

Use radare2 to analyze the original DLL:

```bash
# List all functions
r2 -q -c "aaa; afl" reflex_original.dll

# Search for specific functions
r2 -q -c "aaa; afl~hook" reflex_original.dll

# Disassemble function
r2 -q -c "aaa; pdf @ 0x180001234" reflex_original.dll
```

Calculate offset:
- Function address: `0x180001234`
- DLL base: `0x180000000`
- **Offset**: `0x1234`

Use in code:

```rust
const FUNCTION_OFFSET: usize = 0x1234;
type MyFn = unsafe extern "system" fn(DWORD) -> BOOL;

if let Some(original) = proxy::resolve_internal_function::<MyFn>(FUNCTION_OFFSET) {
    // Call or replace original
}
```

## Documentation

See the parent directory for complete documentation:

- **[../QUICKSTART.md](../QUICKSTART.md)** - 5-minute setup guide
- **[../PROXY_ARCHITECTURE.md](../PROXY_ARCHITECTURE.md)** - Complete architecture guide
- **[../BUILD.md](../BUILD.md)** - Build instructions

## Dependencies

- `winapi` - Windows API bindings
- `log` - Logging facade
- `env_logger` - Logger implementation
- `once_cell` - Lazy static initialization
- `serde` + `toml` - Configuration parsing

## Building on macOS

The code won't run on macOS (Windows-only APIs), but you can verify it compiles:

```bash
# Install Windows target
rustup target add x86_64-pc-windows-gnu

# Build (won't run, but compiles)
cargo build --target x86_64-pc-windows-gnu
```

## Testing on Windows

1. Build the proxy: `cargo build --release`
2. Deploy to application directory
3. Run application
4. Check `reflex.log` for output

## Troubleshooting

### Build Errors

```bash
# Update Rust
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Runtime Errors

Check `reflex.log`:
```bash
type reflex.log
```

Common issues:
- `reflex_original.dll` not found → Make sure it exists
- Original DLL crashes → Check dependencies (hyperkd.sys, etc.)
- Hooks not working → Verify function offsets with radare2

## License

This is a reverse engineering project for educational purposes.
