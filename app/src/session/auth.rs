use super::*;
use std::time::Duration;
const QR_POLL: Duration = Duration::from_millis(1500);
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum QrPhase {
    #[default]
    Idle,
    Pending,
    Scanned,
    Expired,
    Failed,
}
#[derive(Default)]
pub struct AuthState {
    pub open: bool,
    pub account: Option<AccountStatus>,
    pub qr: Option<QrStartResponse>,
    pub phase: QrPhase,
    pub(super) request: Request,
    pub(super) next_poll: Option<Instant>,
    restored: bool,
}
impl AuthState {
    pub fn loading(&self) -> bool {
        self.request.state.loading()
    }
    pub fn error(&self) -> Option<&str> {
        self.request.state.error()
    }
}
impl Session {
    pub fn load_account(&mut self) {
        let id = self.auth.request.begin();
        self.revisions.shell += 1;
        #[cfg(test)]
        if self.test_mode {
            return;
        }
        let core = self.core.clone();
        let inbox = self.inbox.clone();
        self.auth.request.task = Some(self.runtime.spawn(async move {
            inbox.push(AppEvent::AuthStatus(id, commands::auth_status(&core).await));
        }));
    }
    pub(super) fn start_qr(&mut self) {
        self.cancel_qr();
        self.auth.open = true;
        let id = self.auth.request.begin();
        self.revisions.overlay += 1;
        #[cfg(test)]
        if self.test_mode {
            return;
        }
        let core = self.core.clone();
        let inbox = self.inbox.clone();
        self.auth.request.task = Some(self.runtime.spawn(async move {
            inbox.push(AppEvent::QrStarted(
                id,
                commands::auth_qr_start(&core).await,
            ));
        }));
    }
    pub(super) fn poll_qr(&mut self) {
        if !self.auth.open || self.auth.loading() {
            return;
        }
        let Some(qr) = &self.auth.qr else {
            return;
        };
        let session_id = qr.session_id.clone();
        let id = self.auth.request.begin();
        #[cfg(test)]
        if self.test_mode {
            return;
        }
        let core = self.core.clone();
        let inbox = self.inbox.clone();
        self.auth.request.task = Some(self.runtime.spawn(async move {
            inbox.push(AppEvent::QrPolled(
                id,
                commands::auth_qr_poll(&core, session_id).await,
            ));
        }));
    }
    pub(super) fn cancel_qr(&mut self) {
        self.auth.request.cancel();
        if let Err(e) = commands::auth_qr_cancel(&self.core) {
            self.auth.request.state = Operation::Failed(e.message);
        }
        self.auth.qr = None;
        self.auth.phase = QrPhase::Idle;
        self.auth.next_poll = None;
    }
    pub(super) fn logout(&mut self) {
        self.cancel_qr();
        match commands::auth_logout(&self.core) {
            Ok(()) => {
                self.auth.account = None;
                self.auth.open = false;
                self.auth.restored = false;
                self.clear_image(crate::images::ACCOUNT_AVATAR);
                self.disconnect_room();
            }
            Err(error) => self.auth.request.state = Operation::Failed(error.message),
        }
        self.revisions.shell += 1;
        self.revisions.overlay += 1;
        self.revisions.room += 1;
    }
    pub(super) fn on_auth_status(&mut self, id: u64, result: Result<AccountStatus, AppError>) {
        if !self.auth.request.finish(id) {
            return;
        }
        match result {
            Ok(status) => {
                let avatar = status.account.as_ref().and_then(|a| a.avatar_url.clone());
                self.auth.account = Some(status);
                self.request_image(crate::images::ACCOUNT_AVATAR, avatar.as_deref());
                self.restore_room();
            }
            Err(error) => self.auth.request.state = Operation::Failed(error.message),
        }
        self.revisions.shell += 1;
        self.revisions.room += 1;
    }
    pub(super) fn on_qr_started(&mut self, id: u64, result: Result<QrStartResponse, AppError>) {
        if !self.auth.request.finish(id) || !self.auth.open {
            return;
        }
        match result {
            Ok(qr) => {
                self.auth.qr = Some(qr);
                self.auth.phase = QrPhase::Pending;
                self.auth.next_poll = Some(Instant::now() + QR_POLL);
            }
            Err(error) => {
                self.auth.phase = QrPhase::Failed;
                self.auth.request.state = Operation::Failed(error.message);
            }
        }
        self.revisions.overlay += 1;
    }
    pub(super) fn on_qr_polled(&mut self, id: u64, result: Result<AuthPollResponse, AppError>) {
        if !self.auth.request.finish(id) || !self.auth.open {
            return;
        }
        match result {
            Ok(AuthPollResponse::Pending) => {
                self.auth.phase = QrPhase::Pending;
                self.auth.next_poll = Some(Instant::now() + QR_POLL);
            }
            Ok(AuthPollResponse::Scanned) => {
                self.auth.phase = QrPhase::Scanned;
                self.auth.next_poll = Some(Instant::now() + QR_POLL);
            }
            Ok(AuthPollResponse::Expired) => {
                self.auth.qr = None;
                self.auth.phase = QrPhase::Expired;
                self.auth.next_poll = None;
            }
            Ok(AuthPollResponse::Success { account }) => {
                let avatar = account.avatar_url.clone();
                self.auth.account = Some(AccountStatus {
                    authenticated: true,
                    account: Some(account),
                });
                self.auth.open = false;
                self.cancel_qr();
                self.request_image(crate::images::ACCOUNT_AVATAR, avatar.as_deref());
                self.restore_room();
                self.revisions.shell += 1;
                self.revisions.room += 1;
            }
            Err(error) => {
                self.auth.phase = if error.code == commands::ErrorCode::QrExpired {
                    QrPhase::Expired
                } else {
                    QrPhase::Failed
                };
                self.auth.next_poll = None;
                self.auth.request.state = Operation::Failed(error.message);
            }
        }
        self.revisions.overlay += 1;
    }
    fn restore_room(&mut self) {
        if self.authenticated() && !self.auth.restored {
            self.auth.restored = true;
            if !self.room.remembered.is_empty() {
                self.room.input = self.room.remembered.clone();
                self.query_room(false);
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closing_rejects_late_qr_and_account_results() {
        let mut s = Session::for_test();
        s.apply(AppEvent::OpenLogin);
        let id = s.auth.request.revision;
        s.apply(AppEvent::CloseLogin);
        s.apply(AppEvent::QrStarted(
            id,
            Ok(QrStartResponse {
                session_id: "old".into(),
                payload: "https://example.com".into(),
                expires_at: 999,
            }),
        ));
        assert!(s.auth.qr.is_none());
        assert!(s.next_wakeup().is_none());
        s.load_account();
        let id = s.auth.request.revision;
        s.logout();
        s.apply(AppEvent::AuthStatus(
            id,
            Ok(AccountStatus {
                authenticated: true,
                account: Some(nanabobo_core::models::AccountSummary {
                    mid: 1,
                    username: "A".into(),
                    avatar_url: None,
                    level: None,
                    coins: None,
                    bcoin: None,
                    vip: None,
                }),
            }),
        ));
        assert!(!s.authenticated());
    }
}
