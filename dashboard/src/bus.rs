use serde::Deserialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub const DEFAULT_BUS: &str = "/tmp/dotagent-bus";

#[derive(Debug, Clone, Deserialize)]
pub struct BusEvent {
    pub repo: String,
    #[allow(dead_code)]
    pub ts: Option<String>,
    #[allow(dead_code)]
    pub event: Option<String>,
}

pub struct Bus {
    path: PathBuf,
}

impl Bus {
    pub fn open(path: &str) -> Self {
        let p = PathBuf::from(path);
        if !p.exists() {
            let _ = fs::remove_file(&p);
            let c_path = std::ffi::CString::new(p.to_string_lossy().as_bytes()).unwrap();
            unsafe { libc::mkfifo(c_path.as_ptr(), 0o644) };
        }
        Bus { path: p }
    }

    #[allow(dead_code)]
    pub fn write(&self, msg: &str) {
        if self.path.exists() {
            if let Ok(mut f) = OpenOptions::new().append(true).open(&self.path) {
                let _ = f.write_all(format!("{}\n", msg).as_bytes());
            }
        }
    }

    pub fn read_nonblocking(&self) -> Vec<BusEvent> {
        let mut events = Vec::new();
        if !self.path.exists() {
            return events;
        }

        let c_path = std::ffi::CString::new(self.path.to_string_lossy().as_bytes()).unwrap();
        let fd = unsafe { libc::open(c_path.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK) };
        if fd < 0 {
            return events;
        }

        let mut buf = [0u8; 4096];
        let mut total = String::new();
        loop {
            let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                break;
            }
            total.push_str(&String::from_utf8_lossy(&buf[..n as usize]));
        }
        unsafe { libc::close(fd); }

        for line in total.lines() {
            if let Ok(evt) = serde_json::from_str::<BusEvent>(line) {
                events.push(evt);
            }
        }
        events
    }

    pub fn cleanup(&self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        self.cleanup();
    }
}