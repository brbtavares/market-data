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
    pub size: u32,          // size value provided by DLL header
    pub bytes: Vec<u8>,     // raw bytes of the array block (header + entries + footer)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    OfferBookV2 {
        n_action: i32,
        n_position: i32,
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
        date_str: String, // server-formatted date string from DLL
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
    pub seq: u64,                 // monotonic sequence per process
    pub recv_unix_ns: u128,       // SystemTime::now()
    pub recv_mono_ns_from_start: u128, // Instant since start
    pub kind: EventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordFrame {
    Header(FileHeader),
    Event(EventRecord),
}
