# ♪ Music Player TUI

[![Rust](https://img.shields.io/badge/rust-1.85+-de5842?logo=rust)](https://www.rust-lang.org)
[![Crate](https://img.shields.io/badge/version-0.1.1-blue)](https://crates.io/crates/terminal-music-player)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

**A terminal-based music player powered by yt-dlp** — built with Rust, [ratatui](https://github.com/ratatui-org/ratatui), [rodio](https://github.com/RustAudio/rodio). Streams audio from any site supported by yt-dlp into an in-memory buffer. No local files, no disk writes.

---

## Features

- **In-memory streaming** — yt-dlp pipes audio directly into a shared `Vec<u8>` buffer; rodio decodes and plays progressively. Playback starts before the download finishes.
- **Site-agnostic** — works with any URL yt-dlp supports (YouTube, Bilibili, SoundCloud, NicoNico, and hundreds more).
- **Playlist management** — add, delete, rename, and reorder songs. Persisted as JSON in the platform-standard config directory.
- **Quick jump** — press a number key (1–9) to enter song index, press Enter to play. Rapid inputs are coalesced: only the last requested song plays.
- **Playback modes** — Sequential ↺, Repeat One ↺₁, Shuffle ⇄. Cycle with `m`.
- **Position memory** — automatically resumes from where you left off. Press `r` to restart from the beginning.
- **Seek** — `←` / `→` to jump ±10 seconds.
- **Real-time progress** — current / total time displayed with a progress bar.
- **Error dialog** — invalid URLs or download failures show a clear error overlay.
- **Cross-platform** — Linux, macOS, Windows.

---

## Prerequisites

| Tool | Version | Purpose |
|---|---|---|
| [Rust](https://www.rust-lang.org) | ≥ 1.85 (edition 2024) | Build & run |
| [yt-dlp](https://github.com/yt-dlp/yt-dlp) | latest | Audio download from any supported site |

### Install yt-dlp

```bash
# macOS
brew install yt-dlp

# Linux (Debian/Ubuntu)
sudo apt install yt-dlp

# Windows (scoop)
scoop install yt-dlp
```

---

## Installation

### From source (recommended)

```bash
# Clone the repository
git clone https://github.com/dennislan/terminal-music-player.git
cd terminal-music-player

# Build in release mode
cargo build --release

# The binary is at target/release/music-player
```

### From crates.io

```bash
cargo install terminal-music-player
```

---

## Usage

```bash
# Run the TUI
cargo run

# Or use the built binary directly
./target/release/music-player
```

The application opens in a alternate screen. Press `q` to quit.

### Keyboard Shortcuts

#### Playlist navigation & playback

| Key | Action |
|---|---|
| `↑` / `↓` or `k` / `j` | Navigate playlist |
| `Enter` | Play selected song |
| `0`–`9` then `Enter` | Jump to song by number (1-based) |
| `Space` | Pause / Resume |
| `←` / `→` | Seek −10s / +10s |
| `r` | Reset current song to 00:00 |
| `m` | Cycle playback mode (Sequential → Repeat One → Shuffle) |

#### Playlist management

| Key | Action |
|---|---|
| `a` | Add a URL （Recommend to add Bilibili URLs）|
| `e` | Edit selected song's alias |
| `d` | Delete selected song (with confirmation prompt) |
| `Alt+↑` / `Alt+↓` | Move song up / down in the playlist |

#### Other

| Key | Action |
|---|---|
| `q` | Quit (saves position and playlist) |
| `Esc` | Cancel number input / dismiss dialog |

---

## Architecture

```
src/
├── main.rs        Entry point, terminal setup, event loop, key dispatch
├── lib.rs         Library crate root (exposes modules for integration tests)
├── app.rs         Application state (App), screen management, input handling
├── ui.rs          ratatui rendering — Playlist, AddSong, EditAlias, ConfirmDelete, Error
├── stream.rs       yt-dlp subprocess, streaming buffer (Arc<Mutex<Vec<u8>>> + Condvar)
├── player.rs       Background thread, mpsc channels, rodio playback
└── playlist.rs     Song & Playlist structs, JSON persistence
```

### Player Thread Architecture

The **player** runs in a dedicated background thread. The UI communicates with it via mpsc channels:

```
┌──────────┐  Commands (mpsc)   ┌────────────┐
│   TUI    │ ────────────────▶  │  Player    │
│  Event   │ ◀────────────────  │ (thread)   │
│  Loop    │   Events (mpsc)     │            │
└──────────┘                    └─────┬──────┘
                                      │
                               ┌──────▼──────┐
                               │   rodio     │
                               │  (audio)    │
                               └─────────────┘
```

**Commands** (UI → player): `Play(Song)`, `Stop`, `PauseResume`, `SeekTo(f64)`
**Events** (player → UI): `Buffering`, `Started`, `Position`, `Paused`, `Resumed`, `Finished`, `Stopped`, `Error`

### Audio Streaming Flow

```
yt-dlp stdout ──▶ StreamHandle (Arc<Mutex<Vec<u8>>> + Condvar) ──▶ rodio::Decoder ──▶ DeviceSink
```

yt-dlp downloads audio as AAC/m4a and writes it to stdout. The `StreamHandle` accumulates bytes in a shared `Vec<u8>`. A `Condvar` signals the decoder whenever new data arrives. rodio reads progressively — playback begins long before the full download completes.

### Pending Play Cancellation

When the user rapidly inputs multiple song numbers and presses Enter, only the **last** requested song plays. Previous requests are cancelled by overwriting `pending_play_index` before the `Stopped` event triggers the next play.

---

## Playlist Storage

Playlists are saved as JSON in your platform's config directory:

| Platform | Path |
|---|---|
| macOS | `~/Library/Application Support/music-player/playlist.json` |
| Linux | `~/.config/music-player/playlist.json` |
| Windows | `%APPDATA%\music-player\playlist.json` |

Each song stores: `url`, `alias`, and `last_position` (for resume).

---

## Development

### Build & Run

```bash
cargo build          # debug build
cargo check          # type-check only (fastest feedback)
cargo run            # run the TUI (opens in alternate screen)
```

### Test

```bash
cargo test           # run all tests
cargo test -- <name> # run a single test by name
```

### Lint & Format

```bash
cargo clippy         # lint the codebase
cargo fmt           # format code
```

### Release Build

Use the provided build script, which runs all tests before building:

```bash
chmod +x build.sh
./build.sh
```

The script exits with a non-zero code if any test fails, preventing a broken release build.

---

## License

MIT

---

## 📸 Screen Snapshot

<div align="center">

<div style="display: flex; flex-wrap: wrap; justify-content: center; gap: 16px; margin: 0 auto;">

  <div align="center" style="max-width: 380px;">
    <img src="assets/P0.png" alt="Playlist View" style="width: 100%; border-radius: 10px; box-shadow: 0 3px 12px rgba(0,0,0,0.12);"/>
    <p style="margin-top: 8px; font-size: 13px; color: #666;"><strong>Playlist</strong> — Browse and play songs from your playlist</p>
  </div>

  <div align="center" style="max-width: 380px;">
    <img src="assets/P1-Add.png" alt="Add Song View" style="width: 100%; border-radius: 10px; box-shadow: 0 3px 12px rgba(0,0,0,0.12);"/>
    <p style="margin-top: 8px; font-size: 13px; color: #666;"><strong>Add Song</strong> — Paste a URL and set an alias to add a new song</p>
  </div>

  <div align="center" style="max-width: 380px;">
    <img src="assets/P2-Edit.png" alt="Edit Alias View" style="width: 100%; border-radius: 10px; box-shadow: 0 3px 12px rgba(0,0,0,0.12);"/>
    <p style="margin-top: 8px; font-size: 13px; color: #666;"><strong>Edit Alias</strong> — Rename any song in your playlist</p>
  </div>

  <div align="center" style="max-width: 380px;">
    <img src="assets/P3-Split-ClaudeCode.png" alt="Split View - Claude Code" style="width: 100%; border-radius: 10px; box-shadow: 0 3px 12px rgba(0,0,0,0.12);"/>
    <p style="margin-top: 8px; font-size: 13px; color: #666;"><strong>Split View (Claude Code)</strong> — AI-assisted development session</p>
  </div>

  <div align="center" style="max-width: 380px;">
    <img src="assets/P4-Splie-HermesAgent.png" alt="Split View - Hermes Agent" style="width: 100%; border-radius: 10px; box-shadow: 0 3px 12px rgba(0,0,0,0.12);"/>
    <p style="margin-top: 8px; font-size: 13px; color: #666;"><strong>Split View (Hermes Agent)</strong> — Multi-agent collaborative coding</p>
  </div>

</div>

</div>

---

## ☕ Buy Me a Coffee

If you find this project helpful, consider buying me a coffee! Your support keeps the late-night coding fueled. ☕

<div align="center">
  <div style="display: flex; justify-content: center; flex-wrap: wrap; gap: 30px; margin-top: 16px;">
    <div align="center" style="width: 220px; height: 220px; background: #fff; border-radius: 8px; padding: 10px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);">
      <img src="assets/wechat_pay.jpg" alt="WeChat Pay QR Code" style="width: 100%; height: 100%; object-fit: contain; border-radius: 4px;"/>
    </div>
    <div align="center" style="width: 220px; height: 220px; background: #fff; border-radius: 8px; padding: 10px; box-shadow: 0 2px 8px rgba(0,0,0,0.1);">
      <img src="assets/alipay.jpg" alt="AliPay QR Code" style="width: 100%; height: 100%; object-fit: contain; border-radius: 4px;"/>
    </div>
  </div>
  <div style="display: flex; justify-content: center; flex-wrap: wrap; gap: 30px; margin-top: 12px; font-weight: bold;">
    <span style="min-width: 240px;">WeChat Pay</span>
    <span style="min-width: 240px;">AliPay</span>
  </div>
  <p style="margin-top: 20px; color: #888; font-size: 14px;">
    Thank you for your support! 💙
  </p>
</div>

---

## 🌟 Sponsor List

<div align="center">

<p style="color: #666; font-size: 15px; margin-bottom: 28px;">
  Heartfelt thanks to every sponsor — your generosity drives this project forward.
</p>

<div style="display: flex; flex-wrap: wrap; justify-content: center; gap: 20px; margin: 0 auto;">

  <!-- Sponsor Card 1 -->
  <div style="width: 280px; min-width: 240px; flex-shrink: 0; background: linear-gradient(135deg, #f8f9ff 0%, #eef1fb 100%); border-radius: 16px; padding: 22px 20px; box-shadow: 0 4px 16px rgba(0,0,0,0.06), 0 1px 4px rgba(0,0,0,0.04); border: 1px solid rgba(100,120,200,0.08);">
    <div style="display: flex; align-items: center; gap: 14px; margin-bottom: 12px;">
      <div style="width: 48px; height: 48px; border-radius: 50%; background: linear-gradient(135deg, #6c5ce7, #a29bfe); display: flex; align-items: center; justify-content: center; color: #fff; font-size: 20px; font-weight: bold; box-shadow: 0 2px 8px rgba(108,92,231,0.3);">🧑‍💻</div>
      <div>
        <div style="font-weight: 700; font-size: 14.5px; color: #2d3436;">Alice Chen</div>
        <div style="font-size: 13px; color: #e17055; font-weight: 600; margin-top: 2px;">$20.00</div>
      </div>
    </div>
    <p style="font-size: 13px; color: #555; line-height: 1.5; margin: 0;">
      Great terminal music player! Love the clean UI. It's smallest and powerful.
    </p>
  </div>

  <!-- Sponsor Card 2 -->
  <div style="width: 280px; min-width: 240px; flex-shrink: 0; background: linear-gradient(135deg, #fff8f3 0%, #fff0e8 100%); border-radius: 16px; padding: 22px 20px; box-shadow: 0 4px 16px rgba(0,0,0,0.06), 0 1px 4px rgba(0,0,0,0.04); border: 1px solid rgba(230,126,34,0.08);">
    <div style="display: flex; align-items: center; gap: 14px; margin-bottom: 12px;">
      <div style="width: 48px; height: 48px; border-radius: 50%; background: linear-gradient(135deg, #00b894, #55efc4); display: flex; align-items: center; justify-content: center; color: #fff; font-size: 20px; font-weight: bold; box-shadow: 0 2px 8px rgba(0,184,148,0.3);">🎵</div>
      <div>
        <div style="font-weight: 700; font-size: 14.5px; color: #2d3436;">Bob Zhang</div>
        <div style="font-size: 13px; color: #e17055; font-weight: 600; margin-top: 2px;">$10.00</div>
      </div>
    </div>
    <p style="font-size: 13px; color: #555; line-height: 1.5; margin: 0;">
      Exactly what I needed for my headless server setup. Thanks!
    </p>
  </div>

  <!-- Sponsor Card 3 -->
  <div style="width: 280px; min-width: 240px; flex-shrink: 0; background: linear-gradient(135deg, #fef9f0 0%, #fdf2e3 100%); border-radius: 16px; padding: 22px 20px; box-shadow: 0 4px 16px rgba(0,0,0,0.06), 0 1px 4px rgba(0,0,0,0.04); border: 1px solid rgba(245,176,65,0.08);">
    <div style="display: flex; align-items: center; gap: 14px; margin-bottom: 12px;">
      <div style="width: 48px; height: 48px; border-radius: 50%; background: linear-gradient(135deg, #fdcb6e, #ffeaa7); display: flex; align-items: center; justify-content: center; color: #333; font-size: 20px; font-weight: bold; box-shadow: 0 2px 8px rgba(253,203,110,0.3);">⭐</div>
      <div>
        <div style="font-weight: 700; font-size: 14.5px; color: #2d3436;">Carol Wang</div>
        <div style="font-size: 13px; color: #e17055; font-weight: 600; margin-top: 2px;">$50.00</div>
      </div>
    </div>
    <p style="font-size: 13px; color: #555; line-height: 1.5; margin: 0;">
      Amazing! 很久之前我就想在 Terminal 上用音乐播放器，没想到现在竟然有人做出来了。
    </p>
  </div>

</div>

<p style="margin-top: 32px; color: #bbb; font-size: 12.5px; letter-spacing: 0.3px;">
  Want to see your name here? ☕ <a href="#-buy-me-a-coffee" style="color: #6c5ce7; text-decoration: none; font-weight: 600;">Buy me a coffee</a> and let me know!
</p>

</div>
