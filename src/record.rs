//! Durable on-disk schema for captures and replay.
//!
//! Files produced by the recorder are a sequence of framed records:
//! - Each frame is `[len:u32][crc32:u32][payload:len bytes]`.
//! - `payload` is a bincode-serialized [`RecordFrame`].
//! - CRC32 is computed over `payload` only (little-endian fields).
//!
//! The first frame is always [`RecordFrame::Header`], with basic metadata
//! and an estimate of server clock offset versus local time.
//! Subsequent frames are [`RecordFrame::Event`] variants carrying raw
//! L3 Offer Book V2 array blocks and Time & Sales events.
//!
//! This schema is intentionally simple, self-describing, and append-friendly.
//! It allows exact reconstruction and integrity checks during playback.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileHeader {
    pub version: u16,
    pub created_unix_ns: u128,
    pub ticker: String,
    pub exchange: String,
    pub server_clock_offset_ms: i64, // server_time - local_time at start
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawArrayBlock {
    /// Size value provided by DLL header (second i32 in the block header).
    pub size: u32,
    /// Raw bytes of the array block (header + entries + footer).
    /// Layout is `[Q:i32][size:i32][entries...][flags:u32?]` for V2.
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    OfferBookV2 {
    /// Action type (Full/Add/Edit/Delete/DeleteFrom). Kept raw as i32.
        n_action: i32,
    /// Position index relative to the end (0 = last/worst). See player for semantics.
        n_position: i32,
    /// 0 = buys, 1 = sells (convention used by DLL).
        n_side: i32,
        n_qtd: i64,
        n_agent: i32,
        n_offer_id: i64,
        d_price: f64,
        has_price: bool,
        has_qtd: bool,
        has_date: bool,
        has_offer_id: bool,
        has_agent: bool,
        date_str: Option<String>,
        array_sell: Option<RawArrayBlock>,
        array_buy: Option<RawArrayBlock>,
    },
    NewTrade {
    /// Server-formatted date string from DLL.
    date_str: String,
        trade_number: u32,
        price: f64,
        volume: f64,
        qty: i32,
        buy_agent: i32,
        sell_agent: i32,
        trade_type: i32,
        edit_flag: u8,
    },
    HistoryTrade {
        date_str: String,
        trade_number: u32,
        price: f64,
        volume: f64,
        qty: i32,
        buy_agent: i32,
        sell_agent: i32,
        trade_type: i32,
    },
    State { state_type: i32, value: i32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// Monotonic sequence per process.
    pub seq: u64,
    /// System wall-clock time in nanoseconds since UNIX_EPOCH (capture side).
    pub recv_unix_ns: u128,
    /// Monotonic time since process start, in nanoseconds (capture side).
    pub recv_mono_ns_from_start: u128,
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordFrame {
    Header(FileHeader),
    Event(EventRecord),
}
