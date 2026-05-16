use anyhow::{Context, Result};
use std::io::{Read, Seek, SeekFrom};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

pub struct AudioState {
    pub data: Vec<u8>,
    pub finished: bool,
}

pub struct StreamHandle {
    state: Arc<Mutex<AudioState>>,
    cvar: Arc<Condvar>,
    pub pos: usize,
}

impl StreamHandle {
    /// Create a new StreamHandle.
    pub fn new(state: Arc<Mutex<AudioState>>, cvar: Arc<Condvar>, pos: usize) -> Self {
        Self { state, cvar, pos }
    }
}

impl Read for StreamHandle {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut state = self.state.lock().unwrap();
        while self.pos >= state.data.len() && !state.finished {
            state = self.cvar.wait(state).unwrap();
        }
        if self.pos >= state.data.len() {
            return Ok(0);
        }
        let available = state.data.len() - self.pos;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&state.data[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

impl Seek for StreamHandle {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let mut state = self.state.lock().unwrap();

        let target = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(delta) => self.pos as i64 + delta,
            SeekFrom::End(offset) => {
                while !state.finished {
                    state = self.cvar.wait(state).unwrap();
                }
                state.data.len() as i64 + offset
            }
        };

        let target = target.max(0) as usize;

        while target > state.data.len() && !state.finished {
            state = self.cvar.wait(state).unwrap();
        }

        self.pos = target.min(state.data.len());
        Ok(self.pos as u64)
    }
}

unsafe impl Send for StreamHandle {}
unsafe impl Sync for StreamHandle {}

#[derive(Clone)]
pub struct StreamWriter {
    state: Arc<Mutex<AudioState>>,
    cvar: Arc<Condvar>,
}

impl StreamWriter {
    /// Create a new StreamWriter.
    pub fn new(state: Arc<Mutex<AudioState>>, cvar: Arc<Condvar>) -> Self {
        Self { state, cvar }
    }
}

impl StreamWriter {
    pub fn write(&self, data: &[u8]) {
        let mut state = self.state.lock().unwrap();
        state.data.extend_from_slice(data);
        self.cvar.notify_all();
    }

    pub fn finish(&self) {
        let mut state = self.state.lock().unwrap();
        state.finished = true;
        self.cvar.notify_all();
    }
}

pub struct AudioStream {
    pub handle: StreamHandle,
    pub duration_secs: f64,
    pub download_error: Arc<Mutex<Option<String>>>,
}

pub fn get_duration(url: &str) -> Result<f64> {
    let output = Command::new("yt-dlp")
        .args(["--print", "duration", url])
        .output()
        .context("Failed to run yt-dlp")?;

    if !output.status.success() {
        anyhow::bail!("yt-dlp failed to get duration");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let s = stdout.trim();
    if s == "NA" || s.is_empty() {
        return Ok(0.0);
    }
    let duration: f64 = s
        .parse()
        .context("Failed to parse duration from yt-dlp output")?;
    Ok(duration)
}

pub fn create_audio_stream(url: &str) -> Result<AudioStream> {
    let state = Arc::new(Mutex::new(AudioState {
        data: Vec::new(),
        finished: false,
    }));
    let cvar = Arc::new(Condvar::new());

    let handle = StreamHandle {
        state: state.clone(),
        cvar: cvar.clone(),
        pos: 0,
    };

    let writer = StreamWriter {
        state: state.clone(),
        cvar: cvar.clone(),
    };

    let download_error: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let dl_err = download_error.clone();

    let fail_state = state.clone();
    let fail_cvar = cvar.clone();

    let url = url.to_string();

    let duration_secs = get_duration(&url).unwrap_or(0.0);

    let url_clone = url.clone();
    let _join_handle = thread::Builder::new()
        .name("audio-download".into())
        .spawn(move || {
            let result = download_to_buffer(&url_clone, writer);
            if let Err(e) = result {
                *dl_err.lock().unwrap() = Some(format!("Audio download error: {}", e));
                let mut s = fail_state.lock().unwrap();
                s.finished = true;
                fail_cvar.notify_all();
            }
        })
        .context("Failed to spawn download thread")?;

    Ok(AudioStream {
        handle,
        duration_secs,
        download_error,
    })
}

fn download_to_buffer(url: &str, writer: StreamWriter) -> Result<()> {
    let mut child = Command::new("yt-dlp")
        .args([
            "-f",
            "bestaudio[ext=m4a]/bestaudio[ext=mp3]/bestaudio[ext=opus]/bestaudio[ext=ogg]/bestaudio[ext=webm]/bestaudio/best",
            "-o",
            "-",
            "--no-playlist",
            url,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to start yt-dlp")?;

    let mut stdout = child.stdout.take().context("No stdout from yt-dlp")?;
    let mut buf = vec![0u8; 65536];

    loop {
        match stdout.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => writer.write(&buf[..n]),
            Err(e) => {
                anyhow::bail!("Read error from yt-dlp pipe: {}", e);
            }
        }
    }

    let status = child.wait().context("Failed to wait for yt-dlp")?;
    if !status.success() {
        anyhow::bail!("yt-dlp exited with error: {:?}", status.code());
    }

    writer.finish();
    Ok(())
}
