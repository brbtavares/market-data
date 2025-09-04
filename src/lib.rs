//! Market data recorder and player library.
//!
//! This crate provides the core types and logic used by the `market-data`
//! recorder binary and the `player` tool:
//!
//! - `record`: durable on-disk schema (frames, events, raw array blocks)
//! - `book`: Level-3 order book model and parsers to reconstruct state from
//!   ProfitDLL Offer Book V2 raw array blocks, including multi-packet full
//!   book handling and nPosition-from-end semantics
//!
//! The binaries in this repository (`src/main.rs` and `src/bin/player.rs`)
//! use these modules to write and read capture files with strong framing
//! and CRC integrity checks.
pub mod record;
pub mod book;
