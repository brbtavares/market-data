mod ffi;
mod profitdll;
mod record;

use anyhow::{Context, Result};
use clap::Parser;
use dotenvy::dotenv;
use crc32fast::Hasher as Crc32;
use crossbeam_channel::{bounded, Sender};
use once_cell::sync::OnceCell;
use record::{EventKind, EventRecord, FileHeader, RawArrayBlock, RecordFrame};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use widestring::U16CStr;

use crate::ffi::*;
use crate::profitdll::{copy_array_block, to_pwstr, ProfitDll};

#[derive(Debug, Parser)]
#[command(version, about = "L3 OfferBook + Trades recorder (ProfitDLL)")]
struct Args {
    /// Path to ProfitDLL.dll
    #[arg(long, env = "DLL_PATH", default_value = "dll/ProfitDLL.dll")]
    dll: String,

    /// Activation key
    #[arg(long, env = "ACTIVATION_KEY")]
    activation: String,

    /// Username
    #[arg(long, env = "USER")]
    user: String,

    /// Password
    #[arg(long, env = "PASSWORD")]
    password: String,

    /// Ticker symbol (e.g., WINFUT)
    #[arg(long, env = "TICKER")]
    ticker: String,

    /// Exchange code (e.g., F for BMF, B for Bovespa)
    #[arg(long, env = "EXCHANGE")]
    exchange: String,

    /// Output file path (.bin); defaults to captures/TICKER_YYYY_MM_DD.bin
    #[arg(long, env = "OUT_FILE")]
    out: Option<PathBuf>,
}

fn now_unix_ns() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

fn write_frame(w: &mut BufWriter<File>, frame: &RecordFrame) -> Result<()> {
    let payload = bincode::serialize(frame)?;
    let mut hasher = Crc32::new();
    hasher.update(&payload);
    let crc = hasher.finalize();

    let len = payload.len() as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&crc.to_le_bytes())?;
    w.write_all(&payload)?;
    Ok(())
}

fn writer_thread(out: PathBuf, rx: crossbeam_channel::Receiver<RecordFrame>) -> Result<()> {
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).ok();
        }
    }
    let file = OpenOptions::new().create(true).write(true).truncate(true).open(&out)?;
    let mut w = BufWriter::with_capacity(1 << 20, file); // 1 MiB buffer
    for frame in rx {
        write_frame(&mut w, &frame)?;
    }
    w.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    // Load environment variables from .env if present
    let _ = dotenv();
    let args = Args::parse();
    let dll = ProfitDll::load(&args.dll).with_context(|| "Load ProfitDLL.dll")?;

    let (tx, rx) = bounded::<RecordFrame>(8192);
    let tx_writer = tx.clone();

    // Prepare header
    let created_unix_ns = now_unix_ns();
    let start_instant = Instant::now();

    // get server clock offset (best-effort using local timezone)
    // Also capture server Y-M-D if provided for filename default
    let mut server_date: Option<(i32, u8, u8)> = None;
    let server_offset_ms = unsafe {
        use time::{Date, Month, Time as TmTime, UtcOffset, PrimitiveDateTime};
        let mut out = 0i64;
        let mut dt = 0f64; // epoch seconds (if provided)
        let (mut y, mut mo, mut d, mut h, mut mi, mut s, mut ms) = (0, 0, 0, 0, 0, 0, 0);
        let r = (dll.get_server_clock)(&mut dt, &mut y, &mut mo, &mut d, &mut h, &mut mi, &mut s, &mut ms);
        if r == NL_OK && y > 0 && (1..=12).contains(&mo) && d > 0 {
            let month = Month::try_from(mo as u8).unwrap_or(Month::January);
            if let (Ok(date), Ok(time)) = (
                Date::from_calendar_date(y as i32, month, d as u8),
                TmTime::from_hms_milli(h as u8, mi as u8, s as u8, ms as u16),
            ) {
                server_date = Some((y as i32, mo as u8, d as u8));
                let pdt = PrimitiveDateTime::new(date, time);
                if let Ok(offset) = UtcOffset::current_local_offset() {
                    let odt = pdt.assume_offset(offset);
                    let server_ms = odt.unix_timestamp_nanos() / 1_000_000; // i128
                    let local_ms = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as i128;
                    out = (server_ms - local_ms) as i64;
                }
            }
        }
        out
    };

    // Resolve output path
    let out_path = if let Some(p) = args.out.clone() {
        p
    } else {
        // Prefer server date if available, else local date
        let (yy, mm, dd) = if let Some((y, m, d)) = server_date {
            (y, m, d)
        } else if let Ok(now) = time::OffsetDateTime::now_local() {
            let d = now.date();
            (d.year(), d.month() as u8, d.day())
        } else {
            // Fallback: use UNIX date from SystemTime
            let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
            let odt = time::OffsetDateTime::from_unix_timestamp(secs).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
            let d = odt.date();
            (d.year(), d.month() as u8, d.day())
        };
        let fname = format!("{}_{}_{:02}_{:02}.bin", args.ticker.to_uppercase(), yy, mm, dd);
        let mut p = PathBuf::from("captures");
        p.push(fname);
        p
    };

    // spawn writer thread
    std::thread::spawn(move || {
        if let Err(e) = writer_thread(out_path, rx) {
            eprintln!("writer thread error: {e:#}");
        }
    });

    let header = RecordFrame::Header(FileHeader {
        version: 1,
        created_unix_ns,
        ticker: args.ticker.clone(),
        exchange: args.exchange.clone(),
        server_clock_offset_ms: server_offset_ms,
    });
    tx_writer.send(header).ok();

    // Static state used by callbacks
    static TX_CELL: OnceCell<&'static Sender<RecordFrame>> = OnceCell::new();
    static SEQ_CELL: OnceCell<&'static AtomicU64> = OnceCell::new();
    static START_CELL: OnceCell<Instant> = OnceCell::new();
    static FREE_TX_CELL: OnceCell<&'static Sender<(usize, i32)>> = OnceCell::new();

    // Leak small singletons to get 'static references safely
    let tx_static: &'static Sender<RecordFrame> = Box::leak(Box::new(tx.clone()));
    let seq_static: &'static AtomicU64 = Box::leak(Box::new(AtomicU64::new(0)));
    TX_CELL.set(tx_static).ok();
    SEQ_CELL.set(seq_static).ok();
    START_CELL.set(start_instant).ok();

    fn push_event(kind: EventKind) {
        if let (Some(tx), Some(seq), Some(start)) = (TX_CELL.get(), SEQ_CELL.get(), START_CELL.get()) {
            let n = seq.fetch_add(1, Ordering::Relaxed);
            let ev = EventRecord {
                seq: n,
                recv_unix_ns: now_unix_ns(),
                recv_mono_ns_from_start: start.elapsed().as_nanos(),
                kind,
            };
            let _ = tx.send(RecordFrame::Event(ev));
        }
    }

    unsafe extern "system" fn cb_state(n_type: i32, value: i32) {
    push_event(EventKind::State { state_type: n_type, value });
    }

    unsafe extern "system" fn cb_trade(
        _asset: TAssetIDRec,
        pwc_date: PWideChar,
        trade_number: u32,
        price: f64,
        vol: f64,
        qty: i32,
        buy_agent: i32,
        sell_agent: i32,
        trade_type: i32,
        edit: u8,
    ) {
        let date = if !pwc_date.is_null() {
            unsafe { U16CStr::from_ptr_str(pwc_date).to_string_lossy() }
        } else {
            String::new()
        };
    push_event(EventKind::NewTrade {
            date_str: date,
            trade_number,
            price,
            volume: vol,
            qty,
            buy_agent,
            sell_agent,
            trade_type,
            edit_flag: edit,
    });
    }

    unsafe extern "system" fn cb_hist_trade(
        _asset: TAssetIDRec,
        pwc_date: PWideChar,
        trade_number: u32,
        price: f64,
        vol: f64,
        qty: i32,
        buy_agent: i32,
        sell_agent: i32,
        trade_type: i32,
    ) {
        let date = if !pwc_date.is_null() {
            unsafe { U16CStr::from_ptr_str(pwc_date).to_string_lossy() }
        } else {
            String::new()
        };
    push_event(EventKind::HistoryTrade {
            date_str: date,
            trade_number,
            price,
            volume: vol,
            qty,
            buy_agent,
            sell_agent,
            trade_type,
    });
    }

    unsafe extern "system" fn cb_offerbook_v2(
        _asset: TAssetIDRec,
        n_action: i32,
        n_position: i32,
        n_side: i32,
        n_qtd: i64,
        n_agent: i32,
        n_offer_id: i64,
        d_price: f64,
        b_has_price: u8,
        b_has_qtd: u8,
        b_has_date: u8,
        b_has_offer_id: u8,
        b_has_agent: u8,
        pwc_date: PWideChar,
        p_array_sell: *const core::ffi::c_void,
        p_array_buy: *const core::ffi::c_void,
    ) {
        let date = if !pwc_date.is_null() {
            unsafe { U16CStr::from_ptr_str(pwc_date).to_string_lossy() }
        } else {
            String::new()
        };

        // Copy array blocks immediately to avoid lifetime issues; don't call DLL from callback
        // Read size headers (offset 4..8) and copy bytes
    let sell = unsafe { copy_array_block(p_array_sell) }.map(|(size, bytes)| RawArrayBlock { size, bytes });
    let buy = unsafe { copy_array_block(p_array_buy) }.map(|(size, bytes)| RawArrayBlock { size, bytes });
        // Queue FreePointer outside callback if pointers available
    if let Some(tx) = FREE_TX_CELL.get() {
            if !p_array_sell.is_null() {
                // read size
                let p = p_array_sell as *const u8;
                let mut sz = [0u8; 4];
                // safety: header must be at least 8 bytes
                unsafe { sz.copy_from_slice(std::slice::from_raw_parts(p.add(4), 4)); }
                let size = i32::from_le_bytes(sz);
                let _ = tx.send((p_array_sell as usize, size));
            }
            if !p_array_buy.is_null() {
                let p = p_array_buy as *const u8;
                let mut sz = [0u8; 4];
                unsafe { sz.copy_from_slice(std::slice::from_raw_parts(p.add(4), 4)); }
                let size = i32::from_le_bytes(sz);
                let _ = tx.send((p_array_buy as usize, size));
            }
        }

    push_event(EventKind::OfferBookV2 {
            n_action,
            n_position,
            n_side,
            n_qtd,
            n_agent,
            n_offer_id,
            d_price,
            has_price: b_has_price != 0,
            has_qtd: b_has_qtd != 0,
            has_date: b_has_date != 0,
            has_offer_id: b_has_offer_id != 0,
            has_agent: b_has_agent != 0,
            date_str: if date.is_empty() { None } else { Some(date) },
            array_sell: sell,
            array_buy: buy,
    });
    }

    // Register callbacks
    unsafe {
        (dll.set_state_callback)(Some(cb_state));
        (dll.set_trade_callback)(Some(cb_trade));
        (dll.set_history_trade_callback)(Some(cb_hist_trade));
        (dll.set_offer_book_callback_v2)(Some(cb_offerbook_v2));
    }

    // Start FreePointer background thread
    let (free_tx, free_rx) = bounded::<(usize, i32)>(4096);
    let free_tx_static: &'static Sender<(usize, i32)> = Box::leak(Box::new(free_tx.clone()));
    FREE_TX_CELL.set(free_tx_static).ok();
    let free_fn = dll.free_pointer;
    std::thread::spawn(move || {
        while let Ok((ptr_usize, size)) = free_rx.recv() {
            let p = ptr_usize as *mut std::ffi::c_void;
            unsafe { free_fn(p, size) };
        }
    });

    // Initialize Market Login
    let act = to_pwstr(&args.activation);
    let usr = to_pwstr(&args.user);
    let pwd = to_pwstr(&args.password);
    unsafe {
    let ret = (dll.dll_initialize_market_login)(
            act.as_ptr(),
            usr.as_ptr(),
            pwd.as_ptr(),
            Some(cb_state),
            Some(cb_trade),
            None,
            None,
            None,
            Some(cb_hist_trade),
            None,
            None,
        );
        if ret != NL_OK { eprintln!("DLLInitializeMarketLogin returned {}", ret); }
    }

    // Subscribe
    let ticker = to_pwstr(&args.ticker);
    let exch = to_pwstr(&args.exchange);
    unsafe {
        (dll.subscribe_ticker)(ticker.as_ptr(), exch.as_ptr());
        (dll.subscribe_offer_book)(ticker.as_ptr(), exch.as_ptr());
    }

    // Run until Ctrl+C
    // Graceful shutdown: unsubscribe and finalize DLL, then exit
    // Capture only function pointers (Copy) to avoid moving the DLL loader
    let unsub_ob = dll.unsubscribe_offer_book;
    let unsub_tk = dll.unsubscribe_ticker;
    let finalize = dll.dll_finalize;
    let tkr_for_shutdown = to_pwstr(&args.ticker);
    let exc_for_shutdown = to_pwstr(&args.exchange);
    ctrlc::set_handler(move || {
        unsafe {
            (unsub_ob)(tkr_for_shutdown.as_ptr(), exc_for_shutdown.as_ptr());
            (unsub_tk)(tkr_for_shutdown.as_ptr(), exc_for_shutdown.as_ptr());
            (finalize)();
        }
        std::process::exit(0);
    })
    .ok();

    loop {
        std::thread::sleep(Duration::from_millis(250));
    }
}

