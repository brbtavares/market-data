use crc32fast::Hasher as Crc32;
use market_data::book::{parse_block_v2, Book, Entry, OB_LAST_PACKET};
use market_data::record::{EventKind, EventRecord, FileHeader, RawArrayBlock, RecordFrame};
use std::fs::File;
use std::io::{BufWriter, Write, Read, BufReader};

fn write_frame(w: &mut BufWriter<File>, frame: &RecordFrame) {
    let payload = bincode::serialize(frame).unwrap();
    let mut hasher = Crc32::new(); hasher.update(&payload); let crc = hasher.finalize();
    let len = payload.len() as u32;
    w.write_all(&len.to_le_bytes()).unwrap();
    w.write_all(&crc.to_le_bytes()).unwrap();
    w.write_all(&payload).unwrap();
}

#[test]
fn end_to_end_reconstruct_small_book() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.bin");
    let f = File::create(&path).unwrap();
    let mut w = BufWriter::new(f);

    // Header
    write_frame(&mut w, &RecordFrame::Header(FileHeader {
        version: 1,
        created_unix_ns: 0,
        ticker: "TST".into(),
        exchange: "X".into(),
        server_clock_offset_ms: 0,
    }));

    // Full book: buys [101, 100], sells [102, 103]
    let buys = vec![
        Entry { price: 101.0, qty: 2, agent: 1, offer_id: 11, date: None },
        Entry { price: 100.0, qty: 1, agent: 2, offer_id: 12, date: None },
    ];
    let sells = vec![
        Entry { price: 102.0, qty: 1, agent: 3, offer_id: 21, date: None },
        Entry { price: 103.0, qty: 2, agent: 4, offer_id: 22, date: None },
    ];
    let make_block = |es: &Vec<Entry>| -> RawArrayBlock {
        // Craft RawArrayBlock bytes just like book::tests helper
        let mut bytes = Vec::new();
        let q = es.len() as i32;
        bytes.extend_from_slice(&q.to_le_bytes());
        let size_pos = bytes.len();
        bytes.extend_from_slice(&0i32.to_le_bytes());
        for e in es {
            bytes.extend_from_slice(&e.price.to_le_bytes());
            bytes.extend_from_slice(&e.qty.to_le_bytes());
            bytes.extend_from_slice(&e.agent.to_le_bytes());
            bytes.extend_from_slice(&e.offer_id.to_le_bytes());
            let db = e.date.clone().unwrap_or_default().into_bytes();
            let dl = db.len() as i16; bytes.extend_from_slice(&dl.to_le_bytes()); bytes.extend_from_slice(&db);
        }
        bytes.extend_from_slice(&OB_LAST_PACKET.to_le_bytes());
        let size = (bytes.len() as i32).to_le_bytes();
        bytes[size_pos..size_pos+4].copy_from_slice(&size);
        RawArrayBlock { size: es.len() as u32, bytes }
    };

    let (buy_block, _) = (make_block(&buys), OB_LAST_PACKET);
    let (sell_block, _) = (make_block(&sells), OB_LAST_PACKET);

    write_frame(&mut w, &RecordFrame::Event(EventRecord {
        seq: 0,
        recv_unix_ns: 0,
        recv_mono_ns_from_start: 0,
        kind: EventKind::OfferBookV2 {
            n_action: 4,
            n_position: 0,
            n_side: 0,
            n_qtd: 0,
            n_agent: 0,
            n_offer_id: 0,
            d_price: 0.0,
            has_price: false,
            has_qtd: false,
            has_date: false,
            has_offer_id: false,
            has_agent: false,
            date_str: None,
            array_sell: Some(sell_block),
            array_buy: Some(buy_block),
        },
    }));

    // atAdd: add a worse bid (nPosition=0 from end => insert after worst)
    write_frame(&mut w, &RecordFrame::Event(EventRecord {
        seq: 1,
        recv_unix_ns: 0,
        recv_mono_ns_from_start: 0,
        kind: EventKind::OfferBookV2 {
            n_action: 0,
            n_position: 0,
            n_side: 0,
            n_qtd: 1,
            n_agent: 5,
            n_offer_id: 13,
            d_price: 99.5,
            has_price: true,
            has_qtd: true,
            has_date: false,
            has_offer_id: true,
            has_agent: true,
            date_str: None,
            array_sell: None,
            array_buy: None,
        },
    }));

    w.flush().unwrap(); drop(w);

    // Read and reconstruct using the same logic as player
    let f = File::open(&path).unwrap();
    let mut r = BufReader::new(f);
    let mut book = Book::default();
    let mut pend_buy: Vec<Entry> = Vec::new();
    let mut pend_sell: Vec<Entry> = Vec::new();

    // helper read u32
    let read_u32 = |r: &mut BufReader<File>| -> u32 { let mut b=[0u8;4]; r.read_exact(&mut b).unwrap(); u32::from_le_bytes(b) };

    // header
    let len = read_u32(&mut r) as usize; let _crc = read_u32(&mut r); let mut p = vec![0u8;len]; r.read_exact(&mut p).unwrap();
    let _h: RecordFrame = bincode::deserialize(&p).unwrap();

    // full book
    let len = read_u32(&mut r) as usize; let _crc = read_u32(&mut r); let mut p = vec![0u8;len]; r.read_exact(&mut p).unwrap();
    let fr: RecordFrame = bincode::deserialize(&p).unwrap();
    if let RecordFrame::Event(ev) = fr {
        if let EventKind::OfferBookV2 { array_buy, array_sell, n_action, .. } = ev.kind {
            assert_eq!(n_action, 4);
            let (mut b, fb) = parse_block_v2(&array_buy.unwrap()).unwrap(); assert_eq!(fb & OB_LAST_PACKET, OB_LAST_PACKET); pend_buy.append(&mut b); book.apply_full(Some(pend_buy.drain(..).collect()), None);
            let (mut s, fs) = parse_block_v2(&array_sell.unwrap()).unwrap(); assert_eq!(fs & OB_LAST_PACKET, OB_LAST_PACKET); pend_sell.append(&mut s); book.apply_full(None, Some(pend_sell.drain(..).collect()));
        } else { panic!("unexpected kind"); }
    } else { panic!("unexpected frame"); }

    // add
    let len = read_u32(&mut r) as usize; let _crc = read_u32(&mut r); let mut p = vec![0u8;len]; r.read_exact(&mut p).unwrap();
    let fr: RecordFrame = bincode::deserialize(&p).unwrap();
    if let RecordFrame::Event(ev) = fr {
    if let EventKind::OfferBookV2 { n_action, n_side, n_position, d_price, n_qtd, n_agent, n_offer_id, has_price: _ , has_qtd: _ , has_agent: _ , has_offer_id: _ , has_date: _ , date_str, .. } = ev.kind {
            assert_eq!(n_action, 0);
            book.apply_add(n_side, n_position, Entry { price: d_price, qty: n_qtd, agent: n_agent, offer_id: n_offer_id, date: date_str });
        } else { panic!("unexpected kind"); }
    } else { panic!("unexpected frame"); }

    // final assertions
    assert_eq!(book.buys.len(), 3);
    assert!((book.buys[0].price - 101.0).abs() < 1e-9);
    assert!((book.buys[2].price - 99.5).abs() < 1e-9);
    assert_eq!(book.sells.len(), 2);
    assert!((book.sells[0].price - 102.0).abs() < 1e-9);
}

#[test]
fn crc_mismatch_detected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.bin");
    let mut f = BufWriter::new(File::create(&path).unwrap());
    // write one frame with wrong CRC
    let fr = RecordFrame::Header(FileHeader { version:1, created_unix_ns:0, ticker:"X".into(), exchange:"Y".into(), server_clock_offset_ms:0 });
    let payload = bincode::serialize(&fr).unwrap();
    let bad_crc = 0xDEADBEEFu32;
    let len = payload.len() as u32;
    f.write_all(&len.to_le_bytes()).unwrap();
    f.write_all(&bad_crc.to_le_bytes()).unwrap();
    f.write_all(&payload).unwrap();
    f.flush().unwrap();

    let mut r = BufReader::new(File::open(&path).unwrap());
    let mut b=[0u8;4];
    r.read_exact(&mut b).unwrap(); let len = u32::from_le_bytes(b) as usize;
    r.read_exact(&mut b).unwrap(); let crc_file = u32::from_le_bytes(b);
    let mut payload = vec![0u8; len]; r.read_exact(&mut payload).unwrap();
    let mut hasher = Crc32::new(); hasher.update(&payload); let crc_calc = hasher.finalize();
    assert_ne!(crc_calc, crc_file, "CRC should mismatch");
}

#[test]
fn fullbook_multipacket_accumulation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("multi.bin");
    let mut w = BufWriter::new(File::create(&path).unwrap());

    // Header
    write_frame(&mut w, &RecordFrame::Header(FileHeader { version:1, created_unix_ns:0, ticker:"T".into(), exchange:"X".into(), server_clock_offset_ms:0 }));

    // Two packets for buys and sells each
    let mk = |p: f64| Entry { price: p, qty: 1, agent: 1, offer_id: p as i64, date: None };

    // buy packet 1 (not last)
    let b1 = vec![mk(100.0), mk(99.5)];
    // buy packet 2 (last)
    let b2 = vec![mk(99.0)];
    // sell packet 1 (not last)
    let s1 = vec![mk(101.0)];
    // sell packet 2 (last)
    let s2 = vec![mk(101.5), mk(102.0)];

    let make_block = |es: &Vec<Entry>, last: bool| -> RawArrayBlock {
        let mut bytes = Vec::new();
        let q = es.len() as i32; bytes.extend_from_slice(&q.to_le_bytes());
        let size_pos = bytes.len(); bytes.extend_from_slice(&0i32.to_le_bytes());
        for e in es {
            bytes.extend_from_slice(&e.price.to_le_bytes());
            bytes.extend_from_slice(&e.qty.to_le_bytes());
            bytes.extend_from_slice(&e.agent.to_le_bytes());
            bytes.extend_from_slice(&e.offer_id.to_le_bytes());
            let db = e.date.clone().unwrap_or_default().into_bytes();
            let dl = db.len() as i16; bytes.extend_from_slice(&dl.to_le_bytes()); bytes.extend_from_slice(&db);
        }
        let flags = if last { OB_LAST_PACKET } else { 0 }; bytes.extend_from_slice(&flags.to_le_bytes());
        let size = (bytes.len() as i32).to_le_bytes(); bytes[size_pos..size_pos+4].copy_from_slice(&size);
        RawArrayBlock { size: es.len() as u32, bytes }
    };

    // First FullBook frame: buy packet 1, sell packet 1 (both not last)
    write_frame(&mut w, &RecordFrame::Event(EventRecord {
        seq: 0, recv_unix_ns: 0, recv_mono_ns_from_start: 0,
        kind: EventKind::OfferBookV2 { n_action: 4, n_position:0, n_side:0, n_qtd:0, n_agent:0, n_offer_id:0, d_price:0.0,
            has_price:false, has_qtd:false, has_date:false, has_offer_id:false, has_agent:false, date_str: None,
            array_sell: Some(make_block(&s1, false)), array_buy: Some(make_block(&b1, false)) }
    }));

    // Second FullBook frame: buy packet 2 (last), sell packet 2 (last)
    write_frame(&mut w, &RecordFrame::Event(EventRecord {
        seq: 1, recv_unix_ns: 0, recv_mono_ns_from_start: 0,
        kind: EventKind::OfferBookV2 { n_action: 4, n_position:0, n_side:0, n_qtd:0, n_agent:0, n_offer_id:0, d_price:0.0,
            has_price:false, has_qtd:false, has_date:false, has_offer_id:false, has_agent:false, date_str: None,
            array_sell: Some(make_block(&s2, true)), array_buy: Some(make_block(&b2, true)) }
    }));

    w.flush().unwrap(); drop(w);

    // Read and accumulate
    let f = File::open(&path).unwrap();
    let mut r = BufReader::new(f);
    let mut book = Book::default();
    let mut pend_buy: Vec<Entry> = Vec::new();
    let mut pend_sell: Vec<Entry> = Vec::new();

    let read_u32 = |r: &mut BufReader<File>| -> u32 { let mut b=[0u8;4]; r.read_exact(&mut b).unwrap(); u32::from_le_bytes(b) };
    // header
    let len = read_u32(&mut r) as usize; let _ = read_u32(&mut r); let mut p = vec![0u8;len]; r.read_exact(&mut p).unwrap(); let _ : RecordFrame = bincode::deserialize(&p).unwrap();
    // fb1
    let len = read_u32(&mut r) as usize; let _ = read_u32(&mut r); let mut p = vec![0u8;len]; r.read_exact(&mut p).unwrap();
    let fr: RecordFrame = bincode::deserialize(&p).unwrap();
    if let RecordFrame::Event(ev) = fr { if let EventKind::OfferBookV2 { array_buy, array_sell, n_action, .. } = ev.kind {
        assert_eq!(n_action, 4);
        let (mut b, fb) = parse_block_v2(&array_buy.unwrap()).unwrap(); assert_eq!(fb & OB_LAST_PACKET, 0); pend_buy.append(&mut b);
        let (mut s, fs) = parse_block_v2(&array_sell.unwrap()).unwrap(); assert_eq!(fs & OB_LAST_PACKET, 0); pend_sell.append(&mut s);
    } }
    // fb2
    let len = read_u32(&mut r) as usize; let _ = read_u32(&mut r); let mut p = vec![0u8;len]; r.read_exact(&mut p).unwrap();
    let fr: RecordFrame = bincode::deserialize(&p).unwrap();
    if let RecordFrame::Event(ev) = fr { if let EventKind::OfferBookV2 { array_buy, array_sell, .. } = ev.kind {
        let (mut b, fb) = parse_block_v2(&array_buy.unwrap()).unwrap(); assert_eq!(fb & OB_LAST_PACKET, OB_LAST_PACKET); pend_buy.append(&mut b); book.apply_full(Some(pend_buy.drain(..).collect()), None);
        let (mut s, fs) = parse_block_v2(&array_sell.unwrap()).unwrap(); assert_eq!(fs & OB_LAST_PACKET, OB_LAST_PACKET); pend_sell.append(&mut s); book.apply_full(None, Some(pend_sell.drain(..).collect()));
    } }

    // Final checks
    assert_eq!(book.buys.len(), b1.len() + b2.len());
    assert_eq!(book.sells.len(), s1.len() + s2.len());
    assert!((book.buys[0].price - 100.0).abs() < 1e-9);
    assert!((book.buys.last().unwrap().price - 99.0).abs() < 1e-9);
    assert!((book.sells[0].price - 101.0).abs() < 1e-9);
    assert!((book.sells.last().unwrap().price - 102.0).abs() < 1e-9);
}

