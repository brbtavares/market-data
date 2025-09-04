use anyhow::{bail, Context, Result};
use clap::Parser;
use crc32fast::Hasher as Crc32;
use market_data::book::{parse_block_v2, Book, Entry};
use market_data::record::{EventKind, RecordFrame};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;


#[derive(Debug, Parser)]
#[command(about = "Play a recorded L3/Trades log and reconstruct the book")]
struct Args {
    /// Input file path to read (recorded .bin)
    #[arg(long, short = 'i')]
    input: PathBuf,

    /// Dump top-of-book after each update
    #[arg(long, default_value_t = false)]
    dump: bool,

    /// Number of levels to print when dumping
    #[arg(long, default_value_t = 5)]
    top: usize,

    /// Print trades (NewTrade and HistoryTrade) as they are read
    #[arg(long, default_value_t = false)]
    print_trades: bool,
}

fn read_u32<R: Read>(r: &mut R) -> std::io::Result<u32> {
    let mut buf = [0u8;4]; r.read_exact(&mut buf)?; Ok(u32::from_le_bytes(buf))
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut rdr = BufReader::new(File::open(&args.input).with_context(|| format!("open {:?}", args.input))?);
    let mut book = Book::default();
    let mut pend_buy: Vec<Entry> = Vec::new();
    let mut pend_sell: Vec<Entry> = Vec::new();
    let mut frames = 0usize;
    const OB_LAST_PACKET: u32 = 1; // footer flag bit meaning 'last packet' for this block
    loop {
        let len = match read_u32(&mut rdr) {
            Ok(v) => v as usize,
            Err(e) => {
                // EOF or error
                if e.kind() == std::io::ErrorKind::UnexpectedEof { break; } else { return Err(e.into()); }
            }
        };
        let crc_on_file = read_u32(&mut rdr)?;
        let mut payload = vec![0u8; len];
        rdr.read_exact(&mut payload)?;
        let mut hasher = Crc32::new(); hasher.update(&payload); let crc_calc = hasher.finalize();
        if crc_calc != crc_on_file { bail!("CRC mismatch at frame {}: file={:#x}, calc={:#x}", frames, crc_on_file, crc_calc); }
        let frame: RecordFrame = bincode::deserialize(&payload).context("bincode decode")?;
        match frame {
            RecordFrame::Header(h) => {
                frames += 1;
                if args.dump { eprintln!("Header: v{} {}-{} offset_ms={} created={}ns", h.version, h.ticker, h.exchange, h.server_clock_offset_ms, h.created_unix_ns); }
            }
            RecordFrame::Event(ev) => {
                frames += 1;
                match ev.kind {
                    EventKind::OfferBookV2 {
                        n_action,
                        n_position,
                        n_side,
                        n_qtd,
                        n_agent,
                        n_offer_id,
                        d_price,
                        has_price,
                        has_qtd,
                        has_date,
                        has_offer_id,
                        has_agent,
                        date_str,
                        array_sell,
                        array_buy,
                    } => {
                        match n_action {
                            4 => { // atFullBook (may come in multiple packets per side)
                                if let Some(b) = &array_buy {
                                    let (mut entries, flags) = parse_block_v2(b)?;
                                    if (flags & OB_LAST_PACKET) == 0 {
                                        pend_buy.append(&mut entries);
                                    } else {
                                        pend_buy.append(&mut entries);
                                        book.apply_full(Some(std::mem::take(&mut pend_buy)), None);
                                    }
                                }
                                if let Some(s) = &array_sell {
                                    let (mut entries, flags) = parse_block_v2(s)?;
                                    if (flags & OB_LAST_PACKET) == 0 {
                                        pend_sell.append(&mut entries);
                                    } else {
                                        pend_sell.append(&mut entries);
                                        book.apply_full(None, Some(std::mem::take(&mut pend_sell)));
                                    }
                                }
                            }
                            0 => { // atAdd
                                let e = Entry { price: d_price, qty: n_qtd, agent: n_agent, offer_id: n_offer_id, date: date_str };
                                book.apply_add(n_side, n_position, e);
                            }
                            1 => { // atEdit
                                let e = Entry { price: d_price, qty: n_qtd, agent: n_agent, offer_id: n_offer_id, date: date_str };
                                book.apply_edit(n_side, n_position, e, has_price, has_qtd, has_agent, has_offer_id, has_date);
                            }
                            2 => { // atDelete
                                book.apply_delete(n_side, n_position);
                            }
                            3 => { // atDeleteFrom
                                book.apply_delete_from(n_side, n_position);
                            }
                            _ => {}
                        }
                        if args.dump {
                            let tb = book.buys.iter().take(args.top).collect::<Vec<_>>();
                            let ta = book.sells.iter().take(args.top).collect::<Vec<_>>();
                            println!("seq={} action={} side={} pos={} | top{} bids / asks:", ev.seq, n_action, n_side, n_position, args.top);
                            for i in 0..args.top.max(tb.len()).max(ta.len()) {
                                let b = tb.get(i).map(|e| format!("{:>3}: {:>10.2} x {:>7}", i, e.price, e.qty)).unwrap_or_else(|| format!("{:>3}: -", i));
                                let a = ta.get(i).map(|e| format!("{:>10.2} x {:>7}", e.price, e.qty)).unwrap_or_else(|| "-".to_string());
                                println!("{} | {}", b, a);
                            }
                            println!("---");
                        }
                    }
                    EventKind::NewTrade { date_str, trade_number, price, volume, qty, buy_agent, sell_agent, trade_type, edit_flag } => {
                        if args.print_trades {
                            println!(
                                "TRADE new seq={} ts={} num={} price={} qty={} vol={} type={} buy_agent={} sell_agent={} edit={}",
                                ev.seq,
                                date_str,
                                trade_number,
                                price,
                                qty,
                                volume,
                                trade_type,
                                buy_agent,
                                sell_agent,
                                edit_flag
                            );
                        }
                    }
                    EventKind::HistoryTrade { date_str, trade_number, price, volume, qty, buy_agent, sell_agent, trade_type } => {
                        if args.print_trades {
                            println!(
                                "TRADE hist seq={} ts={} num={} price={} qty={} vol={} type={} buy_agent={} sell_agent={}",
                                ev.seq,
                                date_str,
                                trade_number,
                                price,
                                qty,
                                volume,
                                trade_type,
                                buy_agent,
                                sell_agent
                            );
                        }
                    }
                    _ => { /* ignore state */ }
                }
            }
        }
    }
    eprintln!("Read {} frames. Final book: {} bids, {} asks.", frames, book.buys.len(), book.sells.len());
    Ok(())
}
