use super::{AppEvent, Inbox, Session, Snapshot};
use nana_ui::{AppearanceSettings, ThemeMode, WindowMaterialMode};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

const VERSION: u32 = 2;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StoredState {
    pub version: u32,
    pub room_id: String,
    pub snapshots: Vec<Snapshot>,
    pub theme: String,
    pub appearance: AppearanceSettings,
    /// 旧版格式的散落外观字段；读取时合并进 appearance，下次保存移除。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    window_material: Option<WindowMaterialMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backdrop_opacity: Option<f32>,
    pub desktop: super::DesktopDanmakuSettings,
}
impl Default for StoredState {
    fn default() -> Self {
        Self {
            version: VERSION,
            room_id: String::new(),
            snapshots: Vec::new(),
            theme: "dark".into(),
            appearance: AppearanceSettings::default(),
            window_material: None,
            backdrop_opacity: None,
            desktop: super::DesktopDanmakuSettings::default(),
        }
    }
}
impl StoredState {
    pub fn from_session(session: &Session) -> Self {
        Self {
            version: VERSION,
            room_id: session.room.remembered.clone(),
            snapshots: session.stats.snapshots.clone(),
            theme: match session.theme {
                ThemeMode::Dark => "dark",
                ThemeMode::Light => "light",
            }
            .into(),
            appearance: session.appearance,
            window_material: None,
            backdrop_opacity: None,
            desktop: session.desktop.settings.clone(),
        }
    }
    pub fn appearance_settings(&self) -> (ThemeMode, AppearanceSettings) {
        let mut appearance = self.appearance;
        if let Some(material) = self.window_material {
            appearance.set_window_material(material);
        }
        if let Some(opacity) = self.backdrop_opacity {
            appearance.set_backdrop_opacity(opacity);
        }
        (
            if self.theme == "light" {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            },
            appearance,
        )
    }
}

pub struct Loaded {
    pub state: StoredState,
    pub path: PathBuf,
    pub blocked: bool,
    pub error: Option<String>,
}

pub fn load() -> Loaded {
    let local = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|p| PathBuf::from(p).join("AppData/Local")))
        .unwrap_or_else(std::env::temp_dir);
    let path = local.join("NanaBobo/state.json");
    let legacy = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.join("nanabobo-storage.json")));
    load_paths(path, legacy.as_deref())
}

fn load_paths(path: PathBuf, legacy: Option<&Path>) -> Loaded {
    let source = if path.exists() {
        Some(path.as_path())
    } else {
        legacy.filter(|p| p.exists())
    };
    let Some(source) = source else {
        return Loaded {
            state: StoredState::default(),
            path,
            blocked: false,
            error: None,
        };
    };
    let result = fs::read(source)
        .and_then(|bytes| serde_json::from_slice::<StoredState>(&bytes).map_err(io::Error::other));
    match result {
        Ok(mut state) if state.version <= VERSION => {
            state.version = VERSION;
            state.desktop.normalize();
            let migration = source != path && atomic_save(&path, &state).is_err();
            Loaded {
                state,
                path,
                blocked: false,
                error: migration.then(|| "旧数据已读取，但暂时无法保存到新位置。请重试。".into()),
            }
        }
        _ => Loaded {
            state: StoredState::default(),
            path,
            blocked: true,
            error: Some("保存的数据暂时无法读取，原文件已保留。重试时将先备份原文件。".into()),
        },
    }
}

struct Save {
    revision: u64,
    state: StoredState,
    recover: bool,
}
#[derive(Default)]
struct Pending {
    save: Option<Save>,
    closing: bool,
}
pub struct Storage {
    shared: Arc<(Mutex<Pending>, Condvar)>,
    worker: Option<JoinHandle<()>>,
    inbox: Inbox,
}
impl Storage {
    pub fn new(path: PathBuf, mut blocked: bool, inbox: Inbox) -> Self {
        let shared = Arc::new((Mutex::new(Pending::default()), Condvar::new()));
        let worker_shared = shared.clone();
        let worker_inbox = inbox.clone();
        let worker = std::thread::Builder::new()
            .name("nanabobo-storage".into())
            .spawn(move || loop {
                let (lock, signal) = &*worker_shared;
                let mut pending = lock.lock().unwrap_or_else(|e| e.into_inner());
                while pending.save.is_none() && !pending.closing {
                    pending = signal.wait(pending).unwrap_or_else(|e| e.into_inner());
                }
                let Some(save) = pending.save.take() else {
                    break;
                };
                drop(pending);
                let result = save_guarded(&path, &save.state, &mut blocked, save.recover);
                worker_inbox.push(AppEvent::Stored {
                    revision: save.revision,
                    result,
                });
            })
            .ok();
        Self {
            shared,
            worker,
            inbox,
        }
    }
    pub fn save(&self, revision: u64, state: StoredState, recover: bool) {
        if self.worker.is_none() {
            self.inbox.push(AppEvent::Stored {
                revision,
                result: Err("暂时无法保存数据，请重新打开应用后重试。".into()),
            });
            return;
        }
        let (lock, signal) = &*self.shared;
        let mut pending = lock.lock().unwrap_or_else(|e| e.into_inner());
        let recover = recover || pending.save.as_ref().is_some_and(|s| s.recover);
        pending.save = Some(Save {
            revision,
            state,
            recover,
        });
        signal.notify_one();
    }
}
impl Drop for Storage {
    fn drop(&mut self) {
        let (lock, signal) = &*self.shared;
        lock.lock().unwrap_or_else(|e| e.into_inner()).closing = true;
        signal.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn save_guarded(
    path: &Path,
    state: &StoredState,
    blocked: &mut bool,
    recover: bool,
) -> Result<(), String> {
    if *blocked {
        if !recover {
            return Err("原数据已保留，请重试以备份并恢复保存。".into());
        }
        if path.exists() {
            let backup = unique_path(path, "backup");
            fs::copy(path, backup).map_err(|_| "无法备份原数据，原文件未更改。".to_owned())?;
        }
        *blocked = false;
    }
    atomic_save(path, state).map_err(|_| "数据暂时无法保存，请检查可用空间后重试。".to_owned())
}
fn unique_path(path: &Path, suffix: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    path.with_extension(format!("{suffix}-{}-{time}-{serial}", std::process::id()))
}
fn atomic_save(path: &Path, state: &StoredState) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec(state).map_err(io::Error::other)?;
    let temp = unique_path(path, "tmp");
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use nana_ui::BackdropTarget;

    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            let path = unique_path(&std::env::temp_dir().join("nanabobo-storage-test"), "dir");
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn legacy_migration_preserves_source_and_restores_appearance() {
        let temp = Temp::new();
        let legacy = temp.0.join("old.json");
        let target = temp.0.join("new/state.json");
        fs::write(
            &legacy,
            br#"{"room_id":"42","theme":"light","snapshots":[],"window_material":"mica","backdrop_opacity":0.7}"#,
        )
        .unwrap();
        let loaded = load_paths(target.clone(), Some(&legacy));
        assert_eq!(loaded.state.room_id, "42");
        let (theme, appearance) = loaded.state.appearance_settings();
        assert_eq!(theme, ThemeMode::Light);
        assert_eq!(appearance.window_material(), WindowMaterialMode::Mica);
        assert_eq!(appearance.backdrop_opacity(), 0.7);
        assert!(target.exists());
        assert!(legacy.exists());
        assert!(!loaded.blocked);
        let mut appearance = AppearanceSettings::default();
        appearance.set_window_material(WindowMaterialMode::Mica);
        appearance.set_backdrop_opacity(0.7);
        appearance.set_backdrop_target(BackdropTarget::Main);
        appearance.set_titlebar_follows_sidebar(false);
        appearance.set_workspace_corners_enabled(false);
        appearance.set_standard_radius(20.0);
        let state = StoredState {
            appearance,
            ..StoredState::default()
        };
        atomic_save(&target, &state).unwrap();
        let restored = load_paths(target, None).state.appearance_settings().1;
        assert_eq!(restored.window_material(), WindowMaterialMode::Mica);
        assert_eq!(restored.backdrop_opacity(), 0.7);
        assert_eq!(restored.backdrop_target(), BackdropTarget::Main);
        assert!(!restored.titlebar_follows_sidebar());
        assert!(!restored.workspace_corners_enabled());
        assert_eq!(restored.standard_radius(), 20.0);
    }
    #[test]
    fn corrupt_state_is_preserved_until_explicit_recovery_and_backed_up() {
        let temp = Temp::new();
        let target = temp.0.join("state.json");
        fs::write(&target, b"damaged").unwrap();
        let mut loaded = load_paths(target.clone(), None);
        assert!(loaded.blocked);
        assert!(
            save_guarded(&target, &StoredState::default(), &mut loaded.blocked, false).is_err()
        );
        assert_eq!(fs::read(&target).unwrap(), b"damaged");
        save_guarded(&target, &StoredState::default(), &mut loaded.blocked, true).unwrap();
        let backup = fs::read_dir(&temp.0)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p != &target)
            .unwrap();
        assert_eq!(fs::read(backup).unwrap(), b"damaged");
        assert!(!load_paths(target, None).blocked);
    }
    #[test]
    fn drop_flushes_latest_queued_revision() {
        let temp = Temp::new();
        let target = temp.0.join("state.json");
        let inbox = Inbox::new();
        let storage = Storage::new(target.clone(), false, inbox);
        for revision in 1..=20 {
            storage.save(
                revision,
                StoredState {
                    room_id: revision.to_string(),
                    ..StoredState::default()
                },
                false,
            );
        }
        drop(storage);
        assert_eq!(load_paths(target, None).state.room_id, "20");
    }
    #[test]
    fn failed_write_preserves_existing_data() {
        let temp = Temp::new();
        let target = temp.0.join("state.json");
        let original = StoredState {
            room_id: "42".into(),
            ..StoredState::default()
        };
        atomic_save(&target, &original).unwrap();
        let impossible = target.join("child.json");
        assert!(atomic_save(&impossible, &StoredState::default()).is_err());
        assert_eq!(load_paths(target, None).state.room_id, "42");
    }

    #[test]
    fn version_one_adds_default_desktop_settings_and_validates_saved_geometry() {
        let temp = Temp::new();
        let target = temp.0.join("state.json");
        let bytes = br#"{"version":1,"room_id":"42","theme":"light"}"#;
        fs::write(&target, bytes).unwrap();
        let loaded = load_paths(target.clone(), None);
        assert!(!loaded.blocked);
        assert_eq!(loaded.state.version, VERSION);
        assert_eq!(
            loaded.state.desktop,
            super::super::DesktopDanmakuSettings::default()
        );
        assert_eq!(fs::read(&target).unwrap(), bytes);
        let state = StoredState {
            desktop: super::super::DesktopDanmakuSettings {
                width: 20.0,
                height: -10.0,
                font_size: 100.0,
                background_opacity: 2.0,
                position: Some([-400.0, 60.0]),
            },
            ..loaded.state
        };
        atomic_save(&target, &state).unwrap();
        let restored = load_paths(target, None).state.desktop;
        assert_eq!((restored.width, restored.height), (280.0, 240.0));
        assert_eq!(restored.font_size, 36.0);
        assert_eq!(restored.background_opacity, 1.0);
        assert_eq!(restored.position, Some([-400.0, 60.0]));
    }
}
