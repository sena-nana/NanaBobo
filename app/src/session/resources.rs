use super::*;
use crate::images::DecodedImage;
use std::collections::HashMap;
pub enum ImageChange {
    Upload(DecodedImage),
    Remove(String),
}
#[derive(Default)]
struct Slot {
    revision: u64,
    url: Option<String>,
    ready: bool,
    loading: bool,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for Slot {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
#[derive(Default)]
pub(super) struct Resources {
    slots: HashMap<String, Slot>,
    pub pending: Vec<ImageChange>,
}
impl Resources {
    pub fn ready(&self, slot: &str) -> bool {
        self.slots.get(slot).is_some_and(|s| s.ready)
    }
    fn begin(&mut self, name: &str, url: Option<&str>) -> Option<u64> {
        let slot = self.slots.entry(name.into()).or_default();
        let url = url.filter(|url| !url.trim().is_empty()).map(str::to_owned);
        if slot.url == url && (slot.ready || slot.loading) {
            return None;
        }
        if let Some(task) = slot.task.take() {
            task.abort();
        }
        slot.revision += 1;
        slot.ready = false;
        slot.loading = url.is_some();
        slot.url = url;
        self.pending.push(ImageChange::Remove(name.into()));
        slot.loading.then_some(slot.revision)
    }
    pub fn complete(&mut self, name: &str, revision: u64, image: Option<DecodedImage>) -> bool {
        let Some(slot) = self.slots.get_mut(name) else {
            return false;
        };
        if slot.revision != revision || !slot.loading {
            return false;
        }
        slot.loading = false;
        slot.task = None;
        slot.ready = image.is_some();
        if let Some(image) = image {
            self.pending.push(ImageChange::Upload(image));
        }
        true
    }
}
impl Session {
    pub(super) fn clear_image(&mut self, slot: &str) {
        self.resources.begin(slot, None);
    }
    pub(super) fn request_image(&mut self, name: &'static str, url: Option<&str>) {
        let Some(revision) = self.resources.begin(name, url) else {
            return;
        };
        let Some(url) = url.map(str::to_owned) else {
            return;
        };
        #[cfg(test)]
        if self.test_mode {
            return;
        }
        let core = self.core.clone();
        let inbox = self.inbox.clone();
        let task = self.runtime.spawn(async move {
            let image = match commands::fetch_image(&core, &url).await {
                Ok(bytes) => {
                    tokio::task::spawn_blocking(move || crate::images::decode(name, &bytes))
                        .await
                        .ok()
                        .flatten()
                }
                Err(_) => None,
            };
            inbox.push(AppEvent::ImageLoaded {
                slot: name.into(),
                revision,
                image,
            });
        });
        self.resources.slots.get_mut(name).expect("image slot").task = Some(task);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn image() -> DecodedImage {
        DecodedImage {
            slot: "avatar".into(),
            width: 1,
            height: 1,
            rgba: vec![0; 4],
        }
    }
    #[test]
    fn image_identity_rejects_old_results_and_failed_requests_can_retry() {
        let mut r = Resources::default();
        let old = r.begin("avatar", Some("old")).unwrap();
        let new = r.begin("avatar", Some("new")).unwrap();
        assert!(!r.complete("avatar", old, Some(image())));
        assert!(r.complete("avatar", new, None));
        let retry = r.begin("avatar", Some("new")).unwrap();
        assert!(r.complete("avatar", retry, Some(image())));
        r.begin("avatar", None);
        assert!(!r.ready("avatar"));
        assert!(r.begin("avatar", Some("new")).is_some());
    }
}
