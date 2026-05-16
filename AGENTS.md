# terminal-music-player — agent instructions

## Current state

Terminal-based music player powered by yt-dlp. Binary crate, no CI. Has 4 unit tests in `playlist.rs::next_song_index()` and a `tests/` directory for integration tests.

## Toolchain

- **Rust edition 2024** — requires Rust ≥ 1.85. Toolchain installed: **1.95.0**.
- No `rust-toolchain.toml` — falls back to whatever `rustup` default is set system-wide.
- Key dependencies: `ratatui` (TUI), `crossterm` (terminal backend), `rodio` (audio playback), `serde_json` (playlist persistence), `dirs` (config dir).

## External dependency

- **`yt-dlp`** must be installed and on `$PATH`. Used to download audio from supported sites. Install via `brew install yt-dlp`.
- `ffmpeg` is **not** required for this project. yt-dlp downloads AAC/m4a natively via `-f bestaudio` and pipes it to stdout without any remuxing.

## Commands

```bash
cargo build          # debug build
cargo run            # run the binary (opens TUI in alternate screen)
cargo check          # type-check without codegen (fastest feedback)
cargo test           # run all tests
cargo test -- <name> # run a single test by name (e.g., cargo test next_song_index)
cargo clippy         # lint the codebase (catches common mistakes)
cargo fmt            # format code
cargo add <dep>      # add crate dependency
```

## Module structure

| Module | Purpose |
|---|---|
| `main.rs` | Entry point, terminal setup, event loop, key handling |
| `app.rs` | Application state (`App` struct), screen management (`Screen` enum) |
| `ui.rs` | TUI rendering with ratatui — five screens: Playlist, AddSong, EditAlias, ConfirmDelete, Error |
| `stream.rs` | `create_audio_stream()` — runs `yt-dlp` and pipes audio into a streaming buffer |
| `playlist.rs` | `Playlist` / `Song` structs, JSON persistence in platform config dir (macOS: `~/Library/Application Support/terminal-music-player/playlist.json`, Linux: `~/.config/terminal-music-player/playlist.json`, Windows: `%APPDATA%\terminal-music-player\playlist.json`) |
| `player.rs` | `Player` struct — background thread, mpsc channels, rodio playback |

## Architecture notes

- **Player runs in a background thread** communicating via mpsc channels:
  - `PlayerCommand::Play(Song)`, `Stop`, `PauseResume`, `SeekTo(f64)` (UI → player)
  - `PlayerEvent::Buffering`, `Started`, `Paused`, `Resumed`, `Stopped`, `Finished`, `Error` (player → UI)
- **Event timing is critical**: `Buffering` is sent immediately when Play is requested (during download), then `Started` is sent AFTER the download + decode succeeds. This gives the UI a chance to show "Downloading..." feedback during the 10-30 second yt-dlp download.
- **rodio v0.22 API**: `DeviceSinkBuilder::open_default_sink()` → `sink.mixer()` → `rodio::Player::connect_new(mixer)`. No `OutputStream` or `Sink` types — use `rodio::Player` for playback control.
- **Audio flow**: yt-dlp pipes raw audio (m4a/AAC) to stdout → read incrementally into a shared memory buffer (`StreamHandle`). `rodio::Decoder` reads from the buffer via `Read + Seek` (seek returns Unsupported). No temp files — everything stays in memory.
- **Format selection**: `-f bestaudio/best` — prefers audio-only streams but falls back to muxed video+audio if no separate audio track exists. This keeps compatibility with sites (e.g. Tencent Video) that don't offer independent audio formats.
- **Streaming buffer**: `stream.rs::create_audio_stream()` spawns a download thread. A `StreamHandle` backed by `Arc<Mutex<Vec<u8>>>` + `Condvar` provides the data to rodio. The download thread signals the condvar when new data arrives. The decoder blocks on `read()` until data is available.
- **TUI event loop**: polls player events every 500ms (`event::poll(Duration::from_millis(200))`), processes keyboard in between. Player thread sends `Position` events every 500ms for the progress bar.
- **Progress bar**: `Gauge` widget shows current/total time. Total duration is obtained from `yt-dlp --print duration <URL>` before streaming starts.
- **UI text is English** (font boxes for CJK are not handled).

## Key behaviors

- Playlist persists to platform config dir (`{config_dir}/terminal-music-player/playlist.json`) on every add/delete.
- Temp audio files stored in `$TMPDIR/terminal-music-player/`, cleaned up after playback loads.
- Duplicate alias check in `app.rs::add_song()` — rejects if alias already exists.
- `create_audio_stream()` in `stream.rs` spawns a yt-dlp process that downloads audio into an in-memory streaming buffer.

## Constraints

- Binary crate only, no `lib.rs`.
- All text in English — no i18n.
- Requires `yt-dlp` at runtime for audio downloads.
- Limited tests: 4 unit tests in `playlist.rs::next_song_index()`, integration tests in `tests/`. No tests for other modules yet.
- No `build.rs`, no workspace features.
- `.gitignore` only has `/target`.
