use crate::services::settings::SettingsService;
use std::path::Path;
use std::process::Command;

pub struct WallpaperService;

impl WallpaperService {
    /// Set a new wallpaper: copy to ~/.config/background, restart swaybg, regenerate theme
    pub fn set_wallpaper(source_path: &str) {
        let source = Path::new(source_path);
        if !source.exists() {
            eprintln!("[wallpaper] Source path does not exist: {}", source_path);
            return;
        }

        let bg_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".config/background");

        // 1. Copy image to ~/.config/background
        if let Err(e) = std::fs::copy(source, &bg_path) {
            eprintln!("[wallpaper] Failed to copy wallpaper: {}", e);
            return;
        }

        let bg_str = bg_path.to_string_lossy().to_string();

        // 2. Kill existing swaybg
        let _ = Command::new("pkill").arg("swaybg").output();

        // Small delay to let the old process fully exit
        std::thread::sleep(std::time::Duration::from_millis(100));

        // 3. Spawn new swaybg in detached mode
        let bg_path_clone = bg_str.clone();
        crate::services::worker::TaskWorker::dispatch(move || {
            use std::os::unix::process::CommandExt;
            use std::process::Stdio;

            unsafe {
                let mut cmd = Command::new("swaybg");
                cmd.args(["-i", &bg_path_clone, "-m", "fill"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .pre_exec(|| {
                        libc::setsid();
                        Ok(())
                    });
                let _ = cmd.spawn();
            }
        });

        // 4. Update settings.json
        SettingsService::set_wallpaper_path(source_path);

        // 5. Set WALLPAPER env var for matugen and regenerate theme
        std::env::set_var("WALLPAPER", &bg_str);
        crate::services::theme::ThemeService::regenerate();

        eprintln!("[wallpaper] Wallpaper set to: {}", source_path);
    }

    /// Get the currently active wallpaper path
    pub fn get_current_path() -> String {
        SettingsService::get_wallpaper_path()
    }
}
