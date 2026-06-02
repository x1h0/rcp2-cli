use log::{debug, info};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransferStatus {
    Inactive,
    Activating,
    Active,
    Error,
}

#[must_use]
pub fn tools_available() -> bool {
    let check = |name: &str| {
        Command::new("sh")
            .args(["-c", &format!("command -v {name}")])
            .output()
            .is_ok_and(|o| o.status.success())
    };
    check("lsblk") && check("udisksctl")
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

pub struct CopyProgress {
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub current_file: String,
    pub done: bool,
    pub error: Option<String>,
}

pub struct TransferState {
    pub status: TransferStatus,
    pub storage_choice: Option<u32>,
    pub mount_point: Option<String>,
    pub files: Vec<FileEntry>,
    pub current_dir: String,
    pub selected: usize,
    pub message: String,
    pub save_prompt: Option<SavePrompt>,
    pub copy_progress: Option<Arc<Mutex<CopyProgress>>>,
}

pub struct SavePrompt {
    pub source: PathBuf,
    pub input: String,
    pub cursor: usize,
}

impl SavePrompt {
    pub fn input(&mut self, c: char) {
        self.input.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            let prev = self.input[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.input.remove(prev);
            self.cursor = prev;
        }
    }
}

use super::{DEVICE_PAD_PREFIX, format_size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadUploadState {
    Prompting,
    Activating,
    WaitingForMount,
    Copying,
    Deactivating,
    Remounting,
    CreatingNode,
    Done,
}

pub struct PadUpload {
    pub state: PadUploadState,
    pub pad_idx: usize,
    pub source_path: String,
    pub is_new_pad: bool,
    pub pad_name: String,
    pub uploaded_filename: Option<String>,
    pub env_start: Option<f64>,
    pub env_stop: Option<f64>,
    pub prompt: SavePrompt,
    pub message: String,
    pub state_entered_at: Option<std::time::Instant>,
}

impl PadUpload {
    #[must_use]
    pub fn new(pad_idx: usize) -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let input = cwd.to_string_lossy().into_owned() + "/";
        let cursor = input.len();

        PadUpload {
            state: PadUploadState::Prompting,
            pad_idx,
            source_path: String::new(),
            is_new_pad: false,
            pad_name: String::new(),
            uploaded_filename: None,
            env_start: None,
            env_stop: None,
            prompt: SavePrompt {
                source: PathBuf::new(),
                input,
                cursor,
            },
            message: String::new(),
            state_entered_at: None,
        }
    }

    #[must_use]
    pub fn host_target_dir(&self, mount_point: &str) -> PathBuf {
        PathBuf::from(mount_point)
            .join("pads")
            .join((self.pad_idx + 1).to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PadDownloadState {
    Prompting,
    Activating,
    WaitingForMount,
    Copying,
    Deactivating,
    Done,
}

pub struct PadDownload {
    pub state: PadDownloadState,
    pub device_path: String,
    pub save_path: String,
    pub prompt: SavePrompt,
    pub message: String,
}

impl PadDownload {
    #[must_use]
    pub fn new(device_path: &str, pad_name: &str) -> Self {
        let filename = Path::new(device_path).file_name().map_or_else(
            || format!("{pad_name}.wav"),
            |n| n.to_string_lossy().into_owned(),
        );

        let cwd = std::env::current_dir().unwrap_or_default();
        let default_path = cwd.join(&filename);
        let input = default_path.to_string_lossy().into_owned();
        let cursor = input.len();

        PadDownload {
            state: PadDownloadState::Prompting,
            device_path: device_path.to_string(),
            save_path: String::new(),
            prompt: SavePrompt {
                source: PathBuf::new(),
                input,
                cursor,
            },
            message: String::new(),
        }
    }

    #[must_use]
    pub fn host_file_path(&self, mount_point: &str) -> Option<PathBuf> {
        let relative = if let Some(pad_relative) = self.device_path.strip_prefix(DEVICE_PAD_PREFIX) {
            Path::new("pads").join(pad_relative)
        } else {
            PathBuf::from(
                self.device_path
                    .strip_prefix("/Application/emmc-data/")
                    .unwrap_or(&self.device_path),
            )
        };
        if relative.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        }) {
            return None;
        }
        Some(PathBuf::from(mount_point).join(relative))
    }
}

impl Default for TransferState {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferState {
    #[must_use]
    pub fn new() -> Self {
        TransferState {
            status: TransferStatus::Inactive,
            storage_choice: None,
            mount_point: None,
            files: Vec::new(),
            current_dir: "/".into(),
            selected: 0,
            message: String::new(),
            save_prompt: None,
            copy_progress: None,
        }
    }

    pub fn poll_copy(&mut self) {
        let Some(ref progress) = self.copy_progress else {
            return;
        };
        let Ok(p) = progress.lock() else {
            return;
        };
        if p.done {
            if let Some(ref err) = p.error {
                self.message = format!("download failed: {err}");
            } else {
                self.message = format!(
                    "saved {} files ({})",
                    p.files_copied,
                    format_size(p.bytes_copied),
                );
            }
            drop(p);
            self.copy_progress = None;
        } else {
            self.message = format!(
                "copying: {} files ({}) - {}",
                p.files_copied,
                format_size(p.bytes_copied),
                p.current_file,
            );
        }
    }

    #[must_use]
    pub fn is_copying(&self) -> bool {
        self.copy_progress.is_some()
    }

    pub fn find_mount_point(&mut self) -> bool {
        let output = Command::new("lsblk")
            .args(["-J", "-o", "NAME,MOUNTPOINT,VENDOR,MODEL,SIZE,FSTYPE"])
            .output();

        let Ok(output) = output else {
            debug!("lsblk failed");
            return false;
        };

        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout)
            && let Some(devices) = json["blockdevices"].as_array()
        {
            for dev in devices {
                if let Some(mp) = Self::check_device_mount(dev) {
                    self.mount_point = Some(mp);
                    return true;
                }
            }
        }

        self.try_udisks_mount()
    }

    fn is_rodecaster_device(dev: &serde_json::Value) -> bool {
        let vendor = dev["vendor"].as_str().unwrap_or("");
        let model = dev["model"].as_str().unwrap_or("");
        let mp = dev["mountpoint"].as_str().unwrap_or("");

        if vendor.contains("RODE") || model.contains("RODE") || model.contains("Rodecaster") {
            return true;
        }

        if model.contains("File-Stor Gadget") {
            return true;
        }

        if model.contains("MassStorageClass") {
            if mp.contains("RodeCaster") || mp.contains("rodecaster") {
                return true;
            }
            if let Some(children) = dev["children"].as_array() {
                for child in children {
                    let cmp = child["mountpoint"].as_str().unwrap_or("");
                    if cmp.contains("RodeCaster") || cmp.contains("rodecaster") {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn check_device_mount(dev: &serde_json::Value) -> Option<String> {
        if !Self::is_rodecaster_device(dev) {
            return None;
        }

        if let Some(mp) = dev["mountpoint"].as_str()
            && !mp.is_empty()
        {
            info!("found mount point: {mp}");
            return Some(mp.to_string());
        }

        if let Some(children) = dev["children"].as_array() {
            for child in children {
                if let Some(mp) = child["mountpoint"].as_str()
                    && !mp.is_empty()
                {
                    info!("found mount point on partition: {mp}");
                    return Some(mp.to_string());
                }
            }
        }

        None
    }

    fn try_udisks_mount(&mut self) -> bool {
        let output = Command::new("lsblk")
            .args(["-J", "-o", "NAME,VENDOR,MODEL,MOUNTPOINT"])
            .output();

        let Ok(output) = output else { return false };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) else {
            return false;
        };

        let Some(devices) = json["blockdevices"].as_array() else {
            return false;
        };

        for dev in devices {
            if !Self::is_rodecaster_device(dev) {
                let model = dev["model"].as_str().unwrap_or("");
                if !model.contains("MassStorageClass") {
                    continue;
                }
            }

            let name = dev["name"].as_str().unwrap_or("");
            let dev_path = if let Some(children) = dev["children"].as_array() {
                if let Some(child) = children.first() {
                    format!("/dev/{}", child["name"].as_str().unwrap_or(name))
                } else {
                    format!("/dev/{name}")
                }
            } else {
                format!("/dev/{name}")
            };

            info!("attempting udisksctl mount for {dev_path}");
            let result = Command::new("udisksctl")
                .args(["mount", "-b", &dev_path])
                .output();

            if let Ok(out) = result {
                let msg = String::from_utf8_lossy(&out.stdout);
                if let Some(at_idx) = msg.find(" at ") {
                    let mp = msg[at_idx + 4..].trim().trim_end_matches('.');
                    self.mount_point = Some(mp.to_string());
                    info!("mounted via udisksctl: {mp}");
                    return true;
                }
            }
        }
        false
    }

    pub fn refresh_files(&mut self) {
        self.files.clear();
        let Some(ref mp) = self.mount_point else {
            return;
        };

        let base = PathBuf::from(mp);
        let dir_path = if self.current_dir == "/" {
            base.clone()
        } else {
            let relative = self.current_dir.trim_start_matches('/');
            let has_traversal = Path::new(relative)
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir));
            if has_traversal {
                self.current_dir = "/".into();
                base.clone()
            } else {
                base.join(relative)
            }
        };

        if self.current_dir != "/" {
            self.files.push(FileEntry {
                path: PathBuf::from(".."),
                name: "..".into(),
                size: 0,
                is_dir: true,
            });
        }

        let Ok(entries) = std::fs::read_dir(&dir_path) else {
            self.message = format!("cannot read: {}", dir_path.display());
            return;
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let fe = FileEntry {
                path: entry.path(),
                name: entry.file_name().to_string_lossy().into_owned(),
                size: meta.len(),
                is_dir: meta.is_dir(),
            };
            if fe.is_dir {
                dirs.push(fe);
            } else {
                files.push(fe);
            }
        }

        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));

        self.files.extend(dirs);
        self.files.extend(files);
        self.selected = 0;
    }

    pub fn enter_dir(&mut self) {
        let Some(entry) = self.files.get(self.selected) else {
            return;
        };
        if !entry.is_dir {
            return;
        }

        if entry.name == ".." {
            if let Some(pos) = self.current_dir.rfind('/') {
                if pos == 0 {
                    self.current_dir = "/".into();
                } else {
                    self.current_dir = self.current_dir[..pos].to_string();
                }
            }
        } else if self.current_dir == "/" {
            self.current_dir = format!("/{}", entry.name);
        } else {
            self.current_dir = format!("{}/{}", self.current_dir, entry.name);
        }
        self.refresh_files();
    }

    pub fn start_download(&mut self) {
        let Some(entry) = self.files.get(self.selected) else {
            return;
        };
        if entry.is_dir {
            return;
        }

        let cwd = std::env::current_dir().unwrap_or_default();
        let default_path = cwd.join(&entry.name);
        let input = default_path.to_string_lossy().into_owned();
        let cursor = input.len();
        self.save_prompt = Some(SavePrompt {
            source: entry.path.clone(),
            input,
            cursor,
        });
    }

    pub fn confirm_download(&mut self) {
        let Some(prompt) = self.save_prompt.take() else {
            return;
        };
        let dest_path = prompt.input.clone();
        let dest = Path::new(&dest_path);
        if let Some(parent) = dest.parent()
            && !parent.exists()
        {
            self.message = format!("directory does not exist: {}", parent.display());
            return;
        }

        let progress = Arc::new(Mutex::new(CopyProgress {
            files_copied: 0,
            bytes_copied: 0,
            current_file: prompt
                .source
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            done: false,
            error: None,
        }));
        self.copy_progress = Some(progress.clone());
        self.message = "copying...".into();

        let src = prompt.source.clone();
        let dest = dest_path.clone();
        thread::spawn(move || match std::fs::copy(&src, &dest) {
            Ok(bytes) => {
                if let Ok(mut p) = progress.lock() {
                    p.files_copied = 1;
                    p.bytes_copied = bytes;
                    p.done = true;
                }
            }
            Err(e) => {
                if let Ok(mut p) = progress.lock() {
                    p.error = Some(e.to_string());
                    p.done = true;
                }
            }
        });
    }

    pub fn start_dir_download(&mut self) {
        let Some(entry) = self.files.get(self.selected) else {
            return;
        };
        if !entry.is_dir || entry.name == ".." {
            return;
        }

        let cwd = std::env::current_dir().unwrap_or_default();
        let default_path = cwd.join(&entry.name);
        let input = default_path.to_string_lossy().into_owned();
        let cursor = input.len();
        self.save_prompt = Some(SavePrompt {
            source: entry.path.clone(),
            input,
            cursor,
        });
    }

    pub fn confirm_dir_download(&mut self) {
        let Some(prompt) = self.save_prompt.take() else {
            return;
        };
        let src = prompt.source.clone();
        let dest = PathBuf::from(&prompt.input);

        if !src.is_dir() {
            self.message = format!("not a directory: {}", src.display());
            return;
        }

        let progress = Arc::new(Mutex::new(CopyProgress {
            files_copied: 0,
            bytes_copied: 0,
            current_file: String::new(),
            done: false,
            error: None,
        }));
        self.copy_progress = Some(progress.clone());
        self.message = "copying...".into();

        thread::spawn(move || match copy_dir_recursive(&src, &dest, &progress) {
            Ok(()) => {
                if let Ok(mut p) = progress.lock() {
                    p.done = true;
                }
            }
            Err(e) => {
                if let Ok(mut p) = progress.lock() {
                    p.error = Some(e.to_string());
                    p.done = true;
                }
            }
        });
    }

    #[must_use]
    pub fn selected_is_dir(&self) -> bool {
        self.files
            .get(self.selected)
            .is_some_and(|e| e.is_dir && e.name != "..")
    }

    pub fn cancel_download(&mut self) {
        self.save_prompt = None;
    }

    pub fn unmount(&mut self) {
        if let Some(ref mp) = self.mount_point {
            info!("unmounting {mp}");
            let result = Command::new("umount").arg(mp).output();
            if (result.is_err() || result.is_ok_and(|o| !o.status.success()))
                && let Some(dev) = Self::find_block_device_for_mount(mp)
            {
                info!("unmounting via udisksctl: {dev}");
                let _ = Command::new("udisksctl")
                    .args(["unmount", "-b", &dev])
                    .output();
            }
        }
        self.mount_point = None;
        self.files.clear();
    }

    fn find_block_device_for_mount(mount_point: &str) -> Option<String> {
        let output = Command::new("lsblk")
            .args(["-J", "-o", "NAME,MOUNTPOINT"])
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&stdout).ok()?;
        for dev in json["blockdevices"].as_array()? {
            if dev["mountpoint"].as_str() == Some(mount_point) {
                let name = dev["name"].as_str()?;
                return Some(format!("/dev/{name}"));
            }
            if let Some(children) = dev["children"].as_array() {
                for child in children {
                    if child["mountpoint"].as_str() == Some(mount_point) {
                        let name = child["name"].as_str()?;
                        return Some(format!("/dev/{name}"));
                    }
                }
            }
        }
        None
    }
}

fn copy_dir_recursive(
    src: &Path,
    dest: &Path,
    progress: &Arc<Mutex<CopyProgress>>,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;
        let target = dest.join(entry.file_name());

        if entry_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target, progress)?;
        } else {
            if let Ok(mut p) = progress.lock() {
                p.current_file = entry.file_name().to_string_lossy().into_owned();
            }
            let bytes = std::fs::copy(entry.path(), &target)?;
            if let Ok(mut p) = progress.lock() {
                p.files_copied += 1;
                p.bytes_copied += bytes;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_prompt(input: &str, cursor: usize) -> SavePrompt {
        SavePrompt {
            source: PathBuf::new(),
            input: input.to_string(),
            cursor,
        }
    }

    #[test]
    fn prompt_input_appends() {
        let mut p = make_prompt("hello", 5);
        p.input('!');
        assert_eq!(p.input, "hello!");
        assert_eq!(p.cursor, 6);
    }

    #[test]
    fn prompt_input_inserts_at_cursor() {
        let mut p = make_prompt("hllo", 1);
        p.input('e');
        assert_eq!(p.input, "hello");
        assert_eq!(p.cursor, 2);
    }

    #[test]
    fn prompt_backspace_deletes() {
        let mut p = make_prompt("hello", 5);
        p.backspace();
        assert_eq!(p.input, "hell");
        assert_eq!(p.cursor, 4);
    }

    #[test]
    fn prompt_backspace_at_start() {
        let mut p = make_prompt("hello", 0);
        p.backspace();
        assert_eq!(p.input, "hello");
        assert_eq!(p.cursor, 0);
    }

    #[test]
    fn prompt_backspace_unicode() {
        let mut p = make_prompt("héllo", 3);
        p.backspace();
        assert_eq!(p.input, "hllo");
        assert_eq!(p.cursor, 1);
    }

    #[test]
    fn refresh_files_rejects_path_traversal() {
        let dir = std::env::temp_dir().join("rcp2_test_traversal");
        std::fs::create_dir_all(dir.join("sub")).ok();

        let mut state = TransferState::new();
        state.mount_point = Some(dir.to_string_lossy().into_owned());

        state.current_dir = "/../../../etc".into();
        state.refresh_files();
        assert_eq!(state.current_dir, "/");

        state.current_dir = "/sub".into();
        state.refresh_files();
        assert_eq!(state.current_dir, "/sub");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn host_file_path_rejects_traversal_and_absolute() {
        let dl = PadDownload::new("/Application/emmc-data/pads/1/sound.wav", "Pad");
        assert_eq!(
            dl.host_file_path("/mnt"),
            Some(PathBuf::from("/mnt/pads/1/sound.wav"))
        );

        let dl = PadDownload::new("/Application/emmc-data/../../../outside/file", "Pad");
        assert_eq!(dl.host_file_path("/mnt"), None);

        let dl = PadDownload::new("/outside/file", "Pad");
        assert_eq!(dl.host_file_path("/mnt"), None);
    }
}
