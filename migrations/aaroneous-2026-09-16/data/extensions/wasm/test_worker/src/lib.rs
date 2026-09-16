use std::ffi::CString;
use std::os::raw::c_char;

// Import the host function provided by the Hypervisor
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn synapse_write(offset: u32, ptr: *const c_char, len: u32) -> i32;
}

#[cfg(not(target_arch = "wasm32"))]
extern "C" {
    // Native stub for non‑WASM builds
    fn synapse_write(_offset: u32, _ptr: *const c_char, _len: u32) -> i32 {
        // No hypervisor; indicate failure
        -1
    }
}

#[no_mangle]
pub extern "C" fn run() -> i32 {
    // Write "ENZYME_ACTIVE" to the Synapse at offset 300
    let msg = match CString::new("ENZYME_ACTIVE") {
        Ok(c) => c,
        Err(_) => return -1,
    };
    let ptr = msg.as_ptr();
    let len = msg.as_bytes().len() as u32;

    unsafe {
        let result = synapse_write(300, ptr, len);
        if result == 0 {
            1 // Success
        } else {
            result // Error code
        }
    }
}
