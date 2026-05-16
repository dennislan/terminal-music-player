use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex};
use terminal_music_player::stream::{AudioState, StreamHandle, StreamWriter};

#[test]
fn test_stream_handle_read() {
    let state = Arc::new(Mutex::new(AudioState {
        data: vec![1, 2, 3, 4, 5],
        finished: true,
    }));
    let cvar = Arc::new(Condvar::new());
    let mut handle = StreamHandle::new(state.clone(), cvar.clone(), 0);

    let mut buf = [0u8; 3];
    let n = handle.read(&mut buf).unwrap();
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[1, 2, 3]);
    assert_eq!(handle.pos, 3);
}

#[test]
fn test_stream_handle_read_eof() {
    let state = Arc::new(Mutex::new(AudioState {
        data: vec![1, 2, 3],
        finished: true,
    }));
    let cvar = Arc::new(Condvar::new());
    let mut handle = StreamHandle::new(state.clone(), cvar.clone(), 3);

    let mut buf = [0u8; 3];
    let n = handle.read(&mut buf).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn test_stream_handle_seek_start() {
    let state = Arc::new(Mutex::new(AudioState {
        data: vec![1, 2, 3, 4, 5],
        finished: true,
    }));
    let cvar = Arc::new(Condvar::new());
    let mut handle = StreamHandle::new(state.clone(), cvar.clone(), 0);

    let new_pos = handle.seek(SeekFrom::Start(2)).unwrap();
    assert_eq!(new_pos, 2);
    assert_eq!(handle.pos, 2);
}

#[test]
fn test_stream_handle_seek_current() {
    let state = Arc::new(Mutex::new(AudioState {
        data: vec![1, 2, 3, 4, 5],
        finished: true,
    }));
    let cvar = Arc::new(Condvar::new());
    let mut handle = StreamHandle::new(state.clone(), cvar.clone(), 2);

    let new_pos = handle.seek(SeekFrom::Current(1)).unwrap();
    assert_eq!(new_pos, 3);
    assert_eq!(handle.pos, 3);
}

#[test]
fn test_stream_handle_seek_end() {
    let state = Arc::new(Mutex::new(AudioState {
        data: vec![1, 2, 3, 4, 5],
        finished: true,
    }));
    let cvar = Arc::new(Condvar::new());
    let mut handle = StreamHandle::new(state.clone(), cvar.clone(), 0);

    let new_pos = handle.seek(SeekFrom::End(0)).unwrap();
    assert_eq!(new_pos, 5);
    assert_eq!(handle.pos, 5);
}

#[test]
fn test_stream_writer_write() {
    let state = Arc::new(Mutex::new(AudioState {
        data: Vec::new(),
        finished: false,
    }));
    let cvar = Arc::new(Condvar::new());
    let writer = StreamWriter::new(state.clone(), cvar.clone());

    writer.write(&[1, 2, 3]);
    let s = state.lock().unwrap();
    assert_eq!(s.data, vec![1, 2, 3]);
}

#[test]
fn test_stream_writer_finish() {
    let state = Arc::new(Mutex::new(AudioState {
        data: Vec::new(),
        finished: false,
    }));
    let cvar = Arc::new(Condvar::new());
    let writer = StreamWriter::new(state.clone(), cvar.clone());

    writer.finish();
    let s = state.lock().unwrap();
    assert!(s.finished);
}
