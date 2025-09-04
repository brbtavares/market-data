//! Order book model and parsers for ProfitDLL Offer Book V2.
//!
//! This module defines a minimal Level-3 order book representation (`Book` and
//! `Entry`) plus helpers to apply actions emitted by the DLL:
//! - Full book replacement (possibly split into multiple packets per side)
//! - Add/Edit/Delete/DeleteFrom actions
//! - Correct interpretation of `nPosition` as an index from the end
//!
//! The [`parse_block_v2`] function parses the raw array block layout captured
//! by the recorder and returns vectors of entries along with footer flags.
//! The [`OB_LAST_PACKET`] flag indicates that the block completes the current
//! multi-packet transmission for the side.
use anyhow::{bail, Result};
use crate::record::RawArrayBlock;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Price level.
    pub price: f64,
    /// Quantity at that price for this offer.
    pub qty: i64,
    /// Agent/broker ID.
    pub agent: i32,
    /// Unique offer identifier.
    pub offer_id: i64,
    /// Optional server-emitted date string.
    pub date: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Book {
    /// Buy side, best price at index 0.
    pub buys: Vec<Entry>, // index 0 = best bid
    /// Sell side, best price at index 0.
    pub sells: Vec<Entry>, // index 0 = best ask
}

impl Book {
    /// Replace current book sides with the provided vectors (if any).
    pub fn apply_full(&mut self, buy: Option<Vec<Entry>>, sell: Option<Vec<Entry>>) {
        if let Some(b) = buy { self.buys = b; }
        if let Some(s) = sell { self.sells = s; }
    }

    /// Convert `nPosition` (index from end) to zero-based index from start.
    fn index_from_end(len: usize, n_position: i32) -> Option<usize> {
        if n_position < 0 { return None; }
        let pos = n_position as usize;
        if pos >= len { return None; }
        Some(len - pos - 1)
    }

    /// Insert a new entry at a position derived from `nPosition`.
    /// Insert a new entry at a position derived from `nPosition`.
    pub fn apply_add(&mut self, side: i32, n_position: i32, e: Entry) {
        let v = match side { 0 => &mut self.buys, 1 => &mut self.sells, _ => return };
        if v.is_empty() { v.push(e); return; }
        let insert_at = match Self::index_from_end(v.len(), n_position) {
            Some(idx) => (idx + 1).min(v.len()),
            None => v.len(),
        };
        v.insert(insert_at, e);
    }

    /// Edit an existing entry at the position derived from `nPosition`.
    pub fn apply_edit(&mut self, side: i32, n_position: i32, e: Entry, has_price: bool, has_qtd: bool, has_agent: bool, has_offer_id: bool, has_date: bool) {
        let v = match side { 0 => &mut self.buys, 1 => &mut self.sells, _ => return };
        if let Some(idx) = Self::index_from_end(v.len(), n_position) {
            if let Some(cur) = v.get_mut(idx) {
                if has_price { cur.price = e.price; }
                if has_qtd { cur.qty = e.qty; }
                if has_agent { cur.agent = e.agent; }
                if has_offer_id { cur.offer_id = e.offer_id; }
                if has_date { cur.date = e.date; }
            }
        }
    }

    /// Remove a single entry at the position derived from `nPosition`.
    pub fn apply_delete(&mut self, side: i32, n_position: i32) {
        let v = match side { 0 => &mut self.buys, 1 => &mut self.sells, _ => return };
        if let Some(idx) = Self::index_from_end(v.len(), n_position) { let _ = v.remove(idx); }
    }

    /// Remove all entries from the derived index (inclusive) to the end of the side.
    pub fn apply_delete_from(&mut self, side: i32, n_position: i32) {
        let v = match side { 0 => &mut self.buys, 1 => &mut self.sells, _ => return };
        if let Some(idx) = Self::index_from_end(v.len(), n_position) { v.truncate(idx); }
    }
}

/// Offer book footer flag meaning "this block is the last packet for the side".
pub const OB_LAST_PACKET: u32 = 1;

/// Parse an Offer Book V2 raw array block captured from the DLL.
///
/// Returns the list of entries plus the footer flags (if present). The layout
/// is: `[Q:i32][size:i32][entries...][flags:u32?]` where each entry contains
/// `f64 price`, `i64 qty`, `i32 agent`, `i64 offer_id`, `i16 date_len`, then
/// `date_len` bytes for a UTF-8 date string.
pub fn parse_block_v2(block: &RawArrayBlock) -> Result<(Vec<Entry>, u32)> {
    let bytes = &block.bytes;
    if bytes.len() < 8 { bail!("array block too small"); }
    let mut off = 0usize;
    let read_i32 = |b: &[u8], o: &mut usize| -> i32 { let mut tmp = [0u8;4]; tmp.copy_from_slice(&b[*o..*o+4]); *o += 4; i32::from_le_bytes(tmp) };
    let read_i16 = |b: &[u8], o: &mut usize| -> i16 { let mut tmp = [0u8;2]; tmp.copy_from_slice(&b[*o..*o+2]); *o += 2; i16::from_le_bytes(tmp) };
    let read_i64 = |b: &[u8], o: &mut usize| -> i64 { let mut tmp = [0u8;8]; tmp.copy_from_slice(&b[*o..*o+8]); *o += 8; i64::from_le_bytes(tmp) };
    let read_f64 = |b: &[u8], o: &mut usize| -> f64 { let mut tmp = [0u8;8]; tmp.copy_from_slice(&b[*o..*o+8]); *o += 8; f64::from_le_bytes(tmp) };
    let read_u32 = |b: &[u8], o: &mut usize| -> u32 { let mut tmp = [0u8;4]; tmp.copy_from_slice(&b[*o..*o+4]); *o += 4; u32::from_le_bytes(tmp) };

    let q = read_i32(bytes, &mut off);
    let _size = read_i32(bytes, &mut off);
    if q < 0 { bail!("negative entries count"); }
    let q = q as usize;
    let mut out = Vec::with_capacity(q);
    for _ in 0..q {
        if off + 8+8+4+8+2 > bytes.len() { bail!("block truncated while reading entry header"); }
        let price = read_f64(bytes, &mut off);
        let qty = read_i64(bytes, &mut off);
        let agent = read_i32(bytes, &mut off);
        let offer_id = read_i64(bytes, &mut off);
        let date_len = read_i16(bytes, &mut off);
        let date = if date_len > 0 {
            let dl = date_len as usize;
            if off + dl > bytes.len() { bail!("block truncated while reading date"); }
            let s = String::from_utf8_lossy(&bytes[off..off+dl]).to_string();
            off += dl;
            Some(s)
        } else { None };
        out.push(Entry { price, qty, agent, offer_id, date });
    }
    if off + 4 > bytes.len() { bail!("block truncated while reading flags"); }
    let flags = read_u32(bytes, &mut off);
    Ok((out, flags))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_block(entries: &[Entry], last: bool) -> RawArrayBlock {
        let mut bytes = Vec::new();
        let q = entries.len() as i32;
        bytes.extend_from_slice(&q.to_le_bytes());
        // placeholder for size
        let size_pos = bytes.len();
        bytes.extend_from_slice(&0i32.to_le_bytes());
        for e in entries {
            bytes.extend_from_slice(&e.price.to_le_bytes());
            bytes.extend_from_slice(&e.qty.to_le_bytes());
            bytes.extend_from_slice(&e.agent.to_le_bytes());
            bytes.extend_from_slice(&e.offer_id.to_le_bytes());
            let db = e.date.clone().unwrap_or_default().into_bytes();
            let dl = db.len() as i16;
            bytes.extend_from_slice(&dl.to_le_bytes());
            bytes.extend_from_slice(&db);
        }
        let flags = if last { OB_LAST_PACKET } else { 0 };
        bytes.extend_from_slice(&flags.to_le_bytes());
        let size = (bytes.len() as i32).to_le_bytes();
        bytes[size_pos..size_pos+4].copy_from_slice(&size);
        RawArrayBlock { size: entries.len() as u32, bytes }
    }

    #[test]
    fn parse_v2_roundtrip() {
        let es = vec![
            Entry { price: 101.0, qty: 2, agent: 10, offer_id: 1, date: Some("d1".into()) },
            Entry { price: 100.5, qty: 1, agent: 11, offer_id: 2, date: None },
        ];
        let block = make_block(&es, true);
        let (out, flags) = parse_block_v2(&block).unwrap();
        assert_eq!(flags & OB_LAST_PACKET, OB_LAST_PACKET);
        assert_eq!(out, es);
    }

    #[test]
    fn book_actions_apply() {
        let mut b = Book::default();
        b.apply_add(0, 0, Entry { price: 100.0, qty: 1, agent: 1, offer_id: 1, date: None });
        b.apply_add(0, 0, Entry { price: 101.0, qty: 1, agent: 1, offer_id: 2, date: None });
        // nPosition=0 from end => insert after last => becomes worse level
        assert_eq!(b.buys.len(), 2);

        // Edit best (nPosition from end: len-1 -> 1)
        b.apply_edit(0, 1, Entry { price: 102.0, qty: 3, agent: 2, offer_id: 1, date: None }, true, true, true, true, false);
        assert_eq!(b.buys[0].price, 102.0);
        assert_eq!(b.buys[0].qty, 3);

        // Delete worst (nPosition 0)
        b.apply_delete(0, 0);
        assert_eq!(b.buys.len(), 1);

        // Add more and delete from (truncate)
        b.apply_add(0, 0, Entry { price: 99.0, qty: 1, agent: 3, offer_id: 3, date: None });
        b.apply_add(0, 0, Entry { price: 98.0, qty: 1, agent: 3, offer_id: 4, date: None });
        // current buys: [best idx0, ..., worst]
        // nPosition=1 => idx = len-1-1 = len-2; truncate(idx) removes idx..end
        let len = b.buys.len();
        b.apply_delete_from(0, 1);
        assert_eq!(b.buys.len(), len- (len - (len-2))); // remain up to idx-1
    }

    #[test]
    fn edge_cases_out_of_range_and_empty() {
        let mut b = Book::default();
        // delete_from on empty should not panic
        b.apply_delete_from(0, 0);
        assert!(b.buys.is_empty());

        // edit out-of-range should do nothing
        b.apply_edit(0, 5, Entry { price: 1.0, qty: 1, agent: 1, offer_id: 1, date: None }, true, true, true, true, true);
        assert!(b.buys.is_empty());

        // add with out-of-range nPosition should append at end
        b.apply_add(0, 99, Entry { price: 10.0, qty: 1, agent: 1, offer_id: 1, date: None });
        assert_eq!(b.buys.len(), 1);
        assert!((b.buys[0].price - 10.0).abs() < 1e-9);

        // delete out-of-range should do nothing
        b.apply_delete(0, 99);
        assert_eq!(b.buys.len(), 1);
    }
}
