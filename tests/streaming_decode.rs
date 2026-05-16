use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};

/// Mirror of the production stream buffer, but with all data available
/// immediately (no progressive download). Used to test seekable(true)
/// without the progressive delivery variable.
struct BufReader {
    data: Arc<Mutex<Vec<u8>>>,
    pos: usize,
}

impl Read for BufReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let state = self.data.lock().unwrap();
        if self.pos >= state.len() {
            return Ok(0);
        }
        let avail = state.len() - self.pos;
        let to_read = buf.len().min(avail);
        buf[..to_read].copy_from_slice(&state[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

impl Seek for BufReader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let state = self.data.lock().unwrap();
        let target = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(delta) => self.pos as i64 + delta,
            SeekFrom::End(offset) => state.len() as i64 + offset,
        };
        let target = target.max(0) as usize;
        self.pos = target.min(state.len());
        Ok(self.pos as u64)
    }
}

/// Minimal check that seekable(true) doesn't cause a crash during
/// decoder init when all data is present. Uses a raw AAC ADTS frame
/// as input (not a real container, so format detection will fail
/// gracefully — the important thing is no SeekError panic).
#[test]
fn seekable_init_no_panic_on_garbage() {
    let garbage = vec![0u8; 1024];
    let data = Arc::new(Mutex::new(garbage));
    let reader = BufReader { data, pos: 0 };

    let result = rodio::Decoder::builder()
        .with_data(reader)
        .with_seekable(true)
        .build();

    match result {
        Ok(_) => eprintln!("unexpectedly built a decoder from garbage"),
        Err(e) => eprintln!("expected init failure (not a real file): {e}"),
    }
    // The test passes as long as we reach here without a panic.
}

/// Verify that seekable(true) works with a real yt-dlp download where
/// all data is available upfront (no progressive delivery).
#[test]
#[ignore = "requires network (yt-dlp)"]
fn real_file_full_buffer_with_seekable() {
    let url = "https://www.bilibili.com/video/BV1GJ411x7mQ";
    let output = std::process::Command::new("yt-dlp")
        .args(["-f", "bestaudio/best", "-o", "-", "--no-playlist", url])
        .output()
        .expect("yt-dlp must be on PATH");

    assert!(output.status.success());
    assert!(!output.stdout.is_empty());
    let total = output.stdout.len();
    eprintln!("Downloaded {total} bytes");

    let data = Arc::new(Mutex::new(output.stdout));
    let reader = BufReader { data, pos: 0 };

    let decoder = rodio::Decoder::builder()
        .with_data(reader)
        .with_seekable(true)
        .build()
        .expect("Decoder should build");

    let mut count = 0u64;
    for sample in decoder {
        let _ = sample;
        count += 1;
        if count > 500_000 {
            break;
        }
    }
    eprintln!("Decoded {count} samples");
    assert!(count > 100_000, "Expected >100k samples, got {count}");
}
