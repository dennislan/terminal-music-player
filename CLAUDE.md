# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & run

```bash
cargo build          # debug build
cargo run            # run the TUI (opens alternate screen)
cargo check          # type-check only (fastest feedback)
cargo test           # run all tests
```

Binary crate only — no `lib.rs`, no workspace, no `build.rs`. Rust edition 2024 (requires ≥ 1.85).

## External dependency

`yt-dlp` must be on `$PATH` at runtime. It downloads audio from supported URLs via `-f bestaudio -o -` piped to stdout. No ffmpeg required — yt-dlp handles AAC/m4a natively.

## Module structure

| Module | Role |
|---|---|
| `main.rs` | Terminal init, crossterm event loop (200ms poll), key dispatch to per-screen handlers |
| `app.rs` | `App` state struct, `Screen` enum, all playlist/playback operations, event processing |
| `ui.rs` | ratatui rendering — Playlist list, AddSong/EditAlias forms, ConfirmDelete/Error dialogs, progress gauge, status bar |
| `player.rs` | Background thread, mpsc channels, rodio playback via `Player` struct. `AudioFetcher` trait for swappable audio backends |
| `stream.rs` | `create_audio_stream()` — spawns yt-dlp download thread, produces `StreamHandle` (Arc+Mutex+Condvar backed `Read+Seek`) |
| `playlist.rs` | `Song`/`Playlist` structs, JSON persistence, `next_song_index()` utility |

## Architecture

### Player thread

The player runs in a dedicated background thread spawned from `Player::new()`. Communication uses mpsc:

- **UI → player**: `PlayerCommand::Play(Song)`, `Stop`, `PauseResume`, `SeekTo(f64)`
- **Player → UI**: `PlayerEvent::Buffering`, `Started`, `Position`, `Paused`, `Resumed`, `Finished`, `Error`

`Player::with_fetcher(Box<dyn AudioFetcher>)` exists for testing — swap `YtDlpFetcher` with an in-memory mock.

### Streaming buffer

yt-dlp stdout → `download_to_buffer()` thread reads 64KB chunks into `Arc<Mutex<Vec<u8>>>`. Each chunk signals `Condvar`. rodio's `Decoder` (which needs `Read + Seek`) wraps the buffer via `StreamHandle`. Seek returns `Unsupported`. No temp files.

### Event timing

`Buffering` is sent immediately when `Play` is requested (before download starts). `Started` is sent only after download + decode succeed. The UI uses `is_buffering` to show "Downloading..." during the lag. Position events fire every 500ms from the player loop. The TUI event loop polls events with `recv_timeout(500ms)` and keyboard with `event::poll(200ms)`.

### rodio v0.22 API

`DeviceSinkBuilder::open_default_sink()` → `sink.mixer()` → `rodio::Player::connect_new(&mixer)`. Control via `player.play()`, `.pause()`, `.stop()`, `.clear()`, `.try_seek(Duration)`, `.get_pos()`, `.empty()`, `.is_paused()`, `.append(source)`.

### Screen state machine

Five screens: `Playlist` (default), `AddSong`, `EditAlias`, `ConfirmDelete`, `Error`. Overlay screens (`ConfirmDelete`, `Error`) draw the playlist underneath then the dialog on top.

### Playlist persistence

Stored at `{config_dir}/terminal-music-player/playlist.json` (macOS: `~/Library/Application Support`, Linux: `~/.config`, Windows: `%APPDATA%`). Saved on every add/delete/alias edit. `last_position` per song enables resume-on-replay. `last_played_alias` restores selection on restart.

## Key constraints

- No tests for modules other than `playlist.rs` (4 unit tests on `next_song_index`). The `AudioFetcher` trait was designed for testability but isn't yet leveraged.
- The `current_position` in `App` is updated by `PlayerEvent::Position` polls; seeking sets it optimistically before the player confirms.
- UI text is English only — no CJK font handling.
- `status_restore` uses a 1-second timer to show transient messages (play mode switch, seek feedback) then restore the previous status.
