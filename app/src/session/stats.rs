use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub room_id: u64,
    #[serde(default)]
    pub room_name: String,
    pub captured_at: u64,
    pub viewer_count: u64,
    pub follower_count: Option<u64>,
    pub live_status: LiveStatus,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum StatsTab {
    #[default]
    Trend,
    History,
}
pub struct StatsState {
    pub snapshots: Vec<Snapshot>,
    pub selected_room: Option<u64>,
    pub tab: StatsTab,
    pub confirm_clear: Option<u64>,
}
impl StatsState {
    pub fn new(mut snapshots: Vec<Snapshot>) -> Self {
        snapshots.sort_by_key(|s| s.captured_at);
        trim(&mut snapshots);
        Self {
            selected_room: snapshots.last().map(|s| s.room_id),
            snapshots,
            tab: StatsTab::Trend,
            confirm_clear: None,
        }
    }
    pub fn current(&self) -> Vec<&Snapshot> {
        self.snapshots
            .iter()
            .filter(|s| Some(s.room_id) == self.selected_room)
            .collect()
    }
    pub fn rooms(&self) -> Vec<(u64, String)> {
        let mut rooms = std::collections::BTreeMap::new();
        for s in &self.snapshots {
            rooms.insert(
                s.room_id,
                if s.room_name.is_empty() {
                    format!("直播间 {}", s.room_id)
                } else {
                    format!("{} · {}", s.room_name, s.room_id)
                },
            );
        }
        rooms.into_iter().collect()
    }
    pub fn room_label(&self, id: u64) -> String {
        self.rooms()
            .into_iter()
            .find(|(key, _)| *key == id)
            .map(|(_, name)| name)
            .unwrap_or_else(|| format!("直播间 {id}"))
    }
    pub fn viewer_series(&self) -> Vec<(i64, Option<f64>)> {
        self.current()
            .into_iter()
            .map(|s| {
                (
                    (s.captured_at.saturating_mul(1000).min(i64::MAX as u64)) as i64,
                    Some(s.viewer_count as f64),
                )
            })
            .collect()
    }
    pub fn follower_series(&self) -> Vec<(i64, Option<f64>)> {
        self.current()
            .into_iter()
            .map(|s| {
                (
                    (s.captured_at.saturating_mul(1000).min(i64::MAX as u64)) as i64,
                    s.follower_count.map(|v| v as f64),
                )
            })
            .collect()
    }
}
impl Session {
    pub(super) fn record_snapshot(&mut self, info: &RoomInfo) {
        if self
            .stats
            .snapshots
            .iter()
            .any(|s| s.room_id == info.room_id && s.captured_at == info.fetched_at)
        {
            return;
        }
        self.stats.snapshots.push(Snapshot {
            room_id: info.room_id,
            room_name: info.owner_name.clone().unwrap_or_default(),
            captured_at: info.fetched_at,
            viewer_count: info.viewer_count,
            follower_count: info.follower_count,
            live_status: info.live_status,
        });
        trim(&mut self.stats.snapshots);
        self.revisions.stats += 1;
    }
    pub(super) fn clear_stats(&mut self) {
        if let Some(id) = self.stats.confirm_clear.take() {
            self.stats.snapshots.retain(|s| s.room_id != id);
            self.persist(false);
        }
        self.revisions.stats += 1;
        self.revisions.overlay += 1;
    }
}
fn trim(snapshots: &mut Vec<Snapshot>) {
    let mut counts = std::collections::HashMap::new();
    snapshots.reverse();
    snapshots.retain(|s| {
        let n = counts.entry(s.room_id).or_insert(0);
        *n += 1;
        *n <= 2000
    });
    snapshots.reverse();
}
pub fn timestamp(seconds: u64) -> String {
    chrono::DateTime::from_timestamp(seconds.min(i64::MAX as u64) as i64, 0)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|| "时间未知".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_values_and_irregular_times_survive_projection() {
        let s = StatsState::new(vec![
            Snapshot {
                room_id: 1,
                room_name: String::new(),
                captured_at: 10,
                viewer_count: 3,
                follower_count: Some(5),
                live_status: LiveStatus::Live,
            },
            Snapshot {
                room_id: 1,
                room_name: String::new(),
                captured_at: 99,
                viewer_count: 4,
                follower_count: None,
                live_status: LiveStatus::Unknown,
            },
        ]);
        assert_eq!(s.follower_series(), vec![(10000, Some(5.0)), (99000, None)]);
        assert_eq!(s.current().len(), 2);
    }
    #[test]
    fn retention_is_per_room_and_history_does_not_require_authentication() {
        let snapshots = (0..2010)
            .flat_map(|n| {
                [1, 2].map(move |room_id| Snapshot {
                    room_id,
                    room_name: String::new(),
                    captured_at: n,
                    viewer_count: 1,
                    follower_count: None,
                    live_status: LiveStatus::Live,
                })
            })
            .collect();
        let mut s = Session::for_test();
        s.stats = StatsState::new(snapshots);
        assert_eq!(s.stats.snapshots.len(), 4000);
        assert_eq!(s.stats.current().len(), 2000);
        assert!(!s.authenticated());
        assert!(s.room.info.is_none());
    }
}
