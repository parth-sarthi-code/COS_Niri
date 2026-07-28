use crate::services::worker::TaskWorker;
use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

extern "C" {
    fn inotify_init1(flags: i32) -> i32;
    fn inotify_add_watch(fd: i32, pathname: *const i8, mask: u32) -> i32;
    fn read(fd: i32, buf: *mut std::ffi::c_void, count: usize) -> isize;
}

const IN_CLOEXEC: i32 = 0x80000;
const IN_MODIFY: u32 = 0x00000002;

pub struct BrightnessService;

impl BrightnessService {
    /// Get current brightness percentage (0..100) using direct sysfs read with CLI fallback
    pub fn get_brightness() -> u32 {
        if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
            for entry in entries.flatten() {
                let path = entry.path();
                let curr_path = path.join("brightness");
                let max_path = path.join("max_brightness");

                if let (Ok(curr_str), Ok(max_str)) = (
                    fs::read_to_string(&curr_path),
                    fs::read_to_string(&max_path),
                ) {
                    if let (Ok(curr), Ok(max)) = (
                        curr_str.trim().parse::<f32>(),
                        max_str.trim().parse::<f32>(),
                    ) {
                        if max > 0.0 {
                            return ((curr / max) * 100.0).round() as u32;
                        }
                    }
                }
            }
        }

        // Fallback via brightnessctl CLI if sysfs is unreadable
        if let Ok(output) = Command::new("brightnessctl")
            .env("LC_ALL", "C")
            .arg("g")
            .output()
        {
            let curr: f32 = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse()
                .unwrap_or(0.0);

            if let Ok(max_output) = Command::new("brightnessctl")
                .env("LC_ALL", "C")
                .arg("m")
                .output()
            {
                let max: f32 = String::from_utf8_lossy(&max_output.stdout)
                    .trim()
                    .parse()
                    .unwrap_or(1.0);

                if max > 0.0 {
                    return ((curr / max) * 100.0).round() as u32;
                }
            }
        }
        100
    }

    /// Set brightness percentage (0..100) asynchronously using persistent worker thread
    pub fn set_brightness(pct: u32) {
        let val_str = format!("{pct}%");
        TaskWorker::dispatch(move || {
            let _ = Command::new("brightnessctl")
                .env("LC_ALL", "C")
                .args(["set", &val_str])
                .output();
        });
    }

    /// Zero-polling Linux kernel `inotify` event listener for sysfs brightness changes (0.0% CPU)
    pub fn listen_events<F>(mut callback: F)
    where
        F: FnMut(u32) + Send + 'static,
    {
        static LISTENING: AtomicBool = AtomicBool::new(false);
        if LISTENING.swap(true, Ordering::SeqCst) {
            return;
        }

        thread::spawn(move || unsafe {
            let fd = inotify_init1(IN_CLOEXEC);
            if fd < 0 {
                return;
            }

            let mut watched = false;

            if let Ok(entries) = fs::read_dir("/sys/class/backlight") {
                for entry in entries.flatten() {
                    let brightness_file = entry.path().join("brightness");
                    if brightness_file.exists() {
                        let c_path = CString::new(brightness_file.as_os_str().as_bytes()).unwrap();
                        let wd = inotify_add_watch(fd, c_path.as_ptr(), IN_MODIFY);
                        if wd >= 0 {
                            watched = true;
                        }
                    }
                }
            }

            if !watched {
                // Fallback to max_brightness directory watch if specific file failed
                if let Ok(c_path) = CString::new("/sys/class/backlight") {
                    inotify_add_watch(fd, c_path.as_ptr(), IN_MODIFY);
                }
            }

            let mut buffer = [0u8; 1024];

            // Kernel blocks thread on read() with 0.0% CPU until a hardware or software brightness event occurs
            loop {
                let bytes_read = read(fd, buffer.as_mut_ptr() as *mut std::ffi::c_void, buffer.len());
                if bytes_read <= 0 {
                    break;
                }
                let current_brightness = Self::get_brightness();
                callback(current_brightness);
            }
        });
    }
}
