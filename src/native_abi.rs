//! C-compatible Native plugin ABI. All pointers are borrowed for one call.
//! Plugin output must be allocated with the Host callback, then reclaimed by
//! the Host. A plugin instance never owns the library or Host buffers.

use std::ffi::c_void;

pub const NATIVE_ABI_REVISION: u32 = 2;
pub const NATIVE_BOOTSTRAP_SYMBOL: &[u8] = b"ting_plugin_abi_v2";
pub const MAX_NATIVE_CONTROL_BYTES: usize = 1024 * 1024;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeAbiHeader {
    pub abi_revision: u32,
    pub struct_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeHostAllocator {
    pub user_data: *mut c_void,
    pub allocate: unsafe extern "C" fn(*mut c_void, usize) -> *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeTargetInfo {
    pub pointer_width: u32,
    /// 1 = little endian, 2 = big endian.
    pub byte_order: u32,
}

impl NativeTargetInfo {
    pub const CURRENT: Self = Self {
        pointer_width: usize::BITS,
        byte_order: if cfg!(target_endian = "little") { 1 } else { 2 },
    };
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeStatus {
    Ok = 0,
    InvalidInput = -1,
    NoScope = -2,
    NotFound = -3,
    PermissionDenied = -4,
    ResourceLimit = -5,
    Cancelled = -6,
    InternalError = -7,
    UnsupportedOperation = -8,
}

/// Per-call Host table. user_data and all pointers are borrowed until invoke
/// returns; callbacks must not be retained or called from detached threads.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeHostApiV2 {
    pub header: NativeAbiHeader,
    pub target: NativeTargetInfo,
    pub user_data: *mut c_void,
    /// UTF-8 method/JSON in, bounded JSON out in the plugin's borrowed buffer.
    pub invoke: unsafe extern "C" fn(
        *mut c_void,
        *const u8,
        usize,
        *const u8,
        usize,
        *mut u8,
        usize,
        *mut usize,
    ) -> i32,
    /// Resource range -> borrowed binary buffer, actual length, EOF.
    pub read_at: unsafe extern "C" fn(
        *mut c_void,
        *const u8,
        usize,
        u64,
        *mut u8,
        usize,
        *mut usize,
        *mut bool,
    ) -> i32,
    pub write_at: unsafe extern "C" fn(
        *mut c_void,
        *const u8,
        usize,
        u64,
        *const u8,
        usize,
        *mut usize,
    ) -> i32,
    /// Copy binary bytes into a Host chunk lease, return its UTF-8 opaque id.
    pub chunk_create:
        unsafe extern "C" fn(*mut c_void, *const u8, usize, *mut u8, usize, *mut usize) -> i32,
    pub chunk_copy:
        unsafe extern "C" fn(*mut c_void, *const u8, usize, *mut u8, usize, *mut usize) -> i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeCallOutput {
    pub data: *mut u8,
    pub len: usize,
}

impl Default for NativeCallOutput {
    fn default() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativePluginAbiV2 {
    pub header: NativeAbiHeader,
    pub target: NativeTargetInfo,
    pub create: unsafe extern "C" fn() -> *mut c_void,
    pub destroy: unsafe extern "C" fn(*mut c_void),
    /// Query the operation name before registration; the runtime validates
    /// every declared capability operation against this table.
    pub supports: unsafe extern "C" fn(*mut c_void, *const u8, usize) -> bool,
    pub invoke: unsafe extern "C" fn(
        *mut c_void,
        *const u8,
        usize,
        *const u8,
        usize,
        *const NativeHostApiV2,
        NativeHostAllocator,
        *mut NativeCallOutput,
    ) -> i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_abi_has_fixed_revision_and_layout() {
        assert_eq!(NATIVE_ABI_REVISION, 2);
        assert_eq!(std::mem::size_of::<NativeAbiHeader>(), 8);
        assert!(std::mem::size_of::<NativePluginAbiV2>() <= u32::MAX as usize);
    }
}
