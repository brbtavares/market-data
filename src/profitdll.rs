//! Dynamic loader and helpers for ProfitDLL.dll.
//!
//! This module binds the subset of functions required by the recorder using
//! `libloading` at runtime. It also includes small helpers for UTF-16 string
//! conversion and for copying the variable-sized raw array blocks delivered
//! in Offer Book callbacks.
use crate::ffi::*;
use anyhow::{Context, Result};
use libloading::{Library, Symbol};
use std::{ffi::c_void};

/// Thin wrapper holding function pointers resolved from ProfitDLL.dll.
///
/// The `_lib` field keeps the library alive so symbols remain valid.
pub struct ProfitDll {
    _lib: Library,
    pub dll_initialize_market_login: unsafe extern "system" fn(
        PWideChar,
        PWideChar,
        PWideChar,
        TStateCallback,
        TNewTradeCallback,
        TNewDailyCallback,
        TPriceBookCallback,
        TOfferBookCallback,
        THistoryTradeCallback,
        TProgressCallback,
        TTinyBookCallback,
    ) -> i32,
    pub dll_finalize: unsafe extern "system" fn() -> i32,
    pub set_state_callback: unsafe extern "system" fn(TStateCallback) -> i32,
    pub set_offer_book_callback_v2: unsafe extern "system" fn(TOfferBookCallbackV2) -> i32,
    pub set_trade_callback: unsafe extern "system" fn(TNewTradeCallback) -> i32,
    pub set_history_trade_callback: unsafe extern "system" fn(THistoryTradeCallback) -> i32,
    pub subscribe_offer_book: unsafe extern "system" fn(PWideChar, PWideChar) -> i32,
    pub unsubscribe_offer_book: unsafe extern "system" fn(PWideChar, PWideChar) -> i32,
    pub subscribe_ticker: unsafe extern "system" fn(PWideChar, PWideChar) -> i32,
    pub unsubscribe_ticker: unsafe extern "system" fn(PWideChar, PWideChar) -> i32,
    pub get_server_clock: unsafe extern "system" fn(
        *mut f64,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
    ) -> i32,
    pub free_pointer: unsafe extern "system" fn(*mut c_void, i32) -> i32,
}

impl ProfitDll {
    /// Load `ProfitDLL.dll` from the given path and resolve required symbols.
    pub fn load(dll_path: &str) -> Result<Self> {
        unsafe {
            let lib = Library::new(dll_path).with_context(|| format!("loading {}", dll_path))?;
            // Manually type each symbol to the correct signature
            let dllinitialize_market_login: Symbol<unsafe extern "system" fn(PWideChar, PWideChar, PWideChar, TStateCallback, TNewTradeCallback, TNewDailyCallback, TPriceBookCallback, TOfferBookCallback, THistoryTradeCallback, TProgressCallback, TTinyBookCallback) -> i32> = lib.get(b"DLLInitializeMarketLogin").context("missing symbol DLLInitializeMarketLogin")?;
            let dllfinalize: Symbol<unsafe extern "system" fn() -> i32> = lib.get(b"DLLFinalize").context("missing symbol DLLFinalize")?;
            let set_state_callback: Symbol<unsafe extern "system" fn(TStateCallback) -> i32> = lib.get(b"SetStateCallback").context("missing symbol SetStateCallback")?;
            let set_offer_book_callback_v2: Symbol<unsafe extern "system" fn(TOfferBookCallbackV2) -> i32> = lib.get(b"SetOfferBookCallbackV2").context("missing symbol SetOfferBookCallbackV2")?;
            let set_trade_callback: Symbol<unsafe extern "system" fn(TNewTradeCallback) -> i32> = lib.get(b"SetTradeCallback").context("missing symbol SetTradeCallback")?;
            let set_history_trade_callback: Symbol<unsafe extern "system" fn(THistoryTradeCallback) -> i32> = lib.get(b"SetHistoryTradeCallback").context("missing symbol SetHistoryTradeCallback")?;
            let subscribe_offer_book: Symbol<unsafe extern "system" fn(PWideChar, PWideChar) -> i32> = lib.get(b"SubscribeOfferBook").context("missing symbol SubscribeOfferBook")?;
            let unsubscribe_offer_book: Symbol<unsafe extern "system" fn(PWideChar, PWideChar) -> i32> = lib.get(b"UnsubscribeOfferBook").context("missing symbol UnsubscribeOfferBook")?;
            let subscribe_ticker: Symbol<unsafe extern "system" fn(PWideChar, PWideChar) -> i32> = lib.get(b"SubscribeTicker").context("missing symbol SubscribeTicker")?;
            let unsubscribe_ticker: Symbol<unsafe extern "system" fn(PWideChar, PWideChar) -> i32> = lib.get(b"UnsubscribeTicker").context("missing symbol UnsubscribeTicker")?;
            let get_server_clock: Symbol<unsafe extern "system" fn(*mut f64, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> i32> = lib.get(b"GetServerClock").context("missing symbol GetServerClock")?;
            let free_pointer: Symbol<unsafe extern "system" fn(*mut c_void, i32) -> i32> = lib.get(b"FreePointer").context("missing symbol FreePointer")?;

            let this = Self {
                dll_initialize_market_login: *dllinitialize_market_login,
                dll_finalize: *dllfinalize,
                set_state_callback: *set_state_callback,
                set_offer_book_callback_v2: *set_offer_book_callback_v2,
                set_trade_callback: *set_trade_callback,
                set_history_trade_callback: *set_history_trade_callback,
                subscribe_offer_book: *subscribe_offer_book,
                unsubscribe_offer_book: *unsubscribe_offer_book,
                subscribe_ticker: *subscribe_ticker,
                unsubscribe_ticker: *unsubscribe_ticker,
                get_server_clock: *get_server_clock,
                free_pointer: *free_pointer,
                _lib: lib,
            };
            Ok(this)
        }
    }
}

/// Convert a Rust `&str` into a nul-terminated UTF-16 vector suitable for
/// passing as `PWideChar` to the DLL.
pub fn to_pwstr(s: &str) -> Vec<u16> {
    use widestring::U16CString;
    let ws = U16CString::from_str(s).expect("utf16");
    ws.into_vec_with_nul()
}

/// Copy a DLL-provided array block into a Rust-owned Vec<u8>, ensuring the full size is read.
/// Layout starts with 2 x i32 header: [Q, size], then (size-8) bytes follow.
pub unsafe fn copy_array_block(ptr_block: *const c_void) -> Option<(u32, Vec<u8>)> {
    if ptr_block.is_null() {
        return None;
    }
    let p = ptr_block as *const u8;
    // read size at offset 4..8
    let mut size_bytes = [0u8; 4];
    let slice = unsafe { std::slice::from_raw_parts(p.add(4), 4) };
    size_bytes.copy_from_slice(slice);
    let size = u32::from_le_bytes(size_bytes);
    let total = size as usize; // includes header (8) + entries + optional footer
    let bytes = unsafe { std::slice::from_raw_parts(p, total) }.to_vec();
    Some((size, bytes))
}
