# Market L3 + Time&Sales Recorder (ProfitDLL)

This tool records Offer Book L3 (OfferBookCallbackV2) and Time & Trades for a single ticker/exchange via ProfitDLL, producing a robust binary log for exact offline replay.

## Build

- Requires Windows x64 and ProfitDLL.dll available at `dll/ProfitDLL.dll` (default path).
- Rust 1.80+ recommended.

## Run

```powershell
# Build
cargo build
```

You can configure via environment variables in a `.env` file (recommended) or pass CLI flags. Flags override env vars.

### Option A — Using .env (no flags)

1) Create `.env` from `.env.example` and fill values:

```env
DLL_PATH=dll/ProfitDLL.dll
ACTIVATION_KEY=YOUR-ACTIVATION
USER=YOUR-USER
PASSWORD=YOUR-PASS
TICKER=WINFUT
EXCHANGE=F
OUT_FILE=./captures/winfut.bin  # optional; if not set, defaults to captures/TICKER_YYYY_MM_DD.bin
```

2) Run:

```powershell
./target/debug/market_data
```

### Option B — Using flags (override .env)

```powershell
./target/debug/market_data `
  --activation "YOUR-ACTIVATION" `
  --user "YOUR-USER" `
  --password "YOUR-PASS" `
  --ticker WINFUT `
  --exchange F `
  --out .\captures\winfut.bin  # optional
```

Stop with Ctrl+C. The writer creates parent folders if needed and writes length+CRC framed records.

## Output format (binary)

Stream of frames: for each frame
- 4 bytes: payload_len (LE u32)
- 4 bytes: crc32(payload)
- payload: bincode-serialized `RecordFrame`:
  - `Header { version, created_unix_ns, ticker, exchange, server_clock_offset_ms }`
  - `Event { seq, recv_unix_ns, recv_mono_ns_from_start, kind }`

EventKind variants captured:
- OfferBookV2: mirrors the callback parameters and includes raw copies of `pArrayBuy` and `pArraySell` blocks:
  - RawArrayBlock.bytes is an exact copy starting at the block header (Q, size) through entries and footer (OfferBookFlags), ensuring lossless reconstruction.
- NewTrade: real-time trade prints (with server date string)
- HistoryTrade: backfilled trades (when requested)
- State: connection/login state changes

## Replay guidance (exact book reconstruction)

Implement the event loop in your player:
- Maintain two lists for the book: buys and sells. Lists are indexed from best to worst (0 is best).
- For each OfferBookV2:
  - nAction: atAdd=0, atEdit=1, atDelete=2, atDeleteFrom=3, atFullBook=4
  - nSide: 0=Buy, 1=Sell
  - nPosition is counted from the END of the list (manual): physical_index = list.len() - nPosition - 1
  - atFullBook: parse RawArrayBlock.bytes
    - Header: [Q: i32][size: i32]
    - Q entries:
      - price: f64
      - qty: i64
      - agent: i32
      - offer_id: i64
      - date_len: i16
      - date_bytes: [u8; date_len]
    - Footer: [OfferBookFlags: u32] (includes OB_LAST_PACKET=1)
    - Replace your side list with the parsed entries in correct ranking order
  - atAdd: insert after position (from end)
  - atEdit: update at the position (from end)
  - atDelete: remove at the position (from end)
  - atDeleteFrom: truncate from that position (from end)

- Time & Sales:
  - NewTrade and HistoryTrade both include the server date string ("DD/MM/YYYY HH:mm:SS.ZZZ") and recv timestamps. Use seq to preserve arrival order when matching book prints if needed.

## Pitfalls handled

- Threading: callbacks run on DLL ConnectorThread; we do zero heavy work there. Data is copied and pushed to a channel, then a writer thread performs IO.
- No DLL calls inside callbacks: we don’t call DLL APIs from callbacks. Any required FreePointer is done in a separate thread.
- Freeing DLL memory: the arrays received via pArrayBuy/pArraySell are copied, and the original pointer is freed off-thread via `FreePointer` using the size header.
- nPosition semantics: documented above (count from end, i.e., size - nPosition - 1).
- Timestamp & clock skew: we record both the server-formatted date and local monotonic/UNIX receive times. `server_clock_offset_ms` is included as a slot for skew compensation; precise conversion depends on deployment timezone and server epoch.
- Atomic writes: frames are length+CRC, buffered by a single writer thread. On crash, a partial tail frame can be detected and truncated by CRC/length.
- Ordering: each event carries a monotonic `seq` for total ordering within the process.

## Notes

- This binary focuses on recording. A separate player (replaying the log and reconstructing the L3 state) can be added later using the guidance above.
- Channel size and IO buffer are tunable via code (8192 messages, 1MiB BufWriter).
- Consider log rotation for long sessions.

### Output naming

- Default output path: `captures/TICKER_YYYY_MM_DD.bin` (date prefers server date from DLL when available; otherwise local date).
- Override with `--out` flag or `OUT_FILE` env.

### Configuration via environment

These variables are read at startup (via dotenv); CLI flags with the same fields override them:

- DLL_PATH: path to ProfitDLL.dll (default: `dll/ProfitDLL.dll`)
- ACTIVATION_KEY: activation/license key
- USER: login user
- PASSWORD: login password
- TICKER: instrument symbol (e.g., WINFUT)
- EXCHANGE: exchange code (e.g., F or B)
- OUT_FILE: output file path (default: `capture.bin`)

```text
Reconstruction contract
- Input: Recorded frames (RecordFrame)
- Output: Deterministic book state series and time&sales identical to live callback semantics
- Edge cases: multiple FullBooks, OB_LAST_PACKET boundaries, sparse edits, atDeleteFrom, out-of-session transitions
```
