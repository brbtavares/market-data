# Market L3 + Time & Sales Recorder (ProfitDLL)

Compact recorder for Offer Book L3 and Time & Sales via ProfitDLL, plus a player to reconstruct the book exactly from logs.

## Features

- Records L3 Offer Book and Trades for a single ticker/exchange
- Binary framing with length + CRC32 for robust, append-friendly logs
- Exact replay (multi-packet FullBook handling, nPosition semantics)
- Best-effort server clock offset capture for time alignment
- Graceful shutdown: unsubscribe, drain, flush, finalize

## Requirements

- Windows x64
- ProfitDLL.dll accessible (default: `dll/ProfitDLL.dll`)
- Rust 1.80+

## Quick start

```powershell
# Build
cargo build

# Option A: Configure via .env (recommended)
#   Copy .env.example → .env and fill values (activation, user, pass, ticker, exchange)
./target/debug/market_data

# Option B: Flags (override .env)
./target/debug/market_data `
  --activation "YOUR-ACTIVATION" `
  --user "YOUR-USER" `
  --password "YOUR-PASS" `
  --ticker WINFUT `
  --exchange F `
  --out .\captures\winfut.bin

# Stop with Ctrl+C
```

## Configuration

Environment variables (flags override):

- DLL_PATH: path to ProfitDLL.dll (default: `dll/ProfitDLL.dll`)
- ACTIVATION_KEY, USER, PASSWORD
- TICKER, EXCHANGE
- OUT_FILE: output path (default: `captures/TICKER_YYYY_MM_DD.bin`)

## Usage

- Recorder (this binary): writes `captures/TICKER_YYYY_MM_DD.bin` by default.
- Player: reconstructs the book and prints on demand.

```powershell
# Replay a capture and print top-of-book each step
./target/debug/player -i .\captures\WINFUT_2025_09_04.bin --top

# Dump full book snapshots or print trades
./target/debug/player -i .\captures\WINFUT_2025_09_04.bin --dump
./target/debug/player -i .\captures\WINFUT_2025_09_04.bin --print-trades
```

## Output format (binary)

- Frame = `[len:u32][crc32:u32][payload:len bytes]`
- `payload` = bincode of:
  - `Header { version, created_unix_ns, ticker, exchange, server_clock_offset_ms }`
  - `Event { seq, recv_unix_ns, recv_mono_ns_from_start, kind }`
- `EventKind::OfferBookV2` carries raw `RawArrayBlock` for buy/sell with layout:
  - `[Q:i32][size:i32][entries...][flags:u32]` (flags include `OB_LAST_PACKET = 1`)

## Replay semantics (player)

- nAction: atAdd=0, atEdit=1, atDelete=2, atDeleteFrom=3, atFullBook=4
- nSide: 0=Buy, 1=Sell
- nPosition: counted from END → index = len - nPosition - 1
- FullBook can arrive split per side; apply when `OB_LAST_PACKET` is set

## Graceful shutdown

- Ctrl+C → unsubscribe ticker/book → short wait → stop enqueuing → drain writer → flush → finalize DLL

## License

Dual-licensed under either:

- Apache License, Version 2.0 (LICENSE-APACHE)
- MIT license (LICENSE-MIT)

at your option.

Contributions are accepted under the same dual license.
