//! NanaBobo L3 宿主：Rust 控件直接写入 NanaUI RuntimeDocument。

mod images;
mod session;
mod ui;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use nana_ui::runtime::{DocumentId, RuntimeDocument};
use nana_ui::{
    run_runtime, HostTextureRegistry, RuntimeProgram, RuntimeProgramContext, RuntimeProgramUpdate,
    RuntimeWindowSettings, ThemeMode,
};
use nana_ui_platform::{WindowEvent, WindowId};

use crate::images::DecodedImage;
use crate::session::{AppEvent, Session, Wake};
use crate::ui::Shell;

fn main() -> Result<(), nana_ui::HostedRunError> {
    run_runtime::<NanaBoboProgram>(
        RuntimeWindowSettings::new("Nana播播工具箱")
            .initial_size(1200.0, 800.0)
            .minimum_size(960.0, 600.0)
            .system_caption(false),
    )
}

struct NanaBoboProgram {
    _runtime: tokio::runtime::Runtime,
    document: RuntimeDocument,
    shell: Shell,
    session: Session,
    textures: HostTextureRegistry,
    decoded: HashMap<String, DecodedImage>,
    gpu_keep: HashMap<String, wgpu::Texture>,
    next_texture_id: u64,
}

impl NanaBoboProgram {
    fn apply_pending(&mut self) {
        let events = self.session.inbox.drain();
        if events.is_empty() {
            return;
        }
        for event in events {
            self.session.apply(event);
        }
        let _ = self.shell.sync(&mut self.document, &self.session);
    }

    fn upload_image(&mut self, context: &RuntimeProgramContext<Wake>, image: &DecodedImage) {
        self.next_texture_id = self.next_texture_id.saturating_add(1);
        let keep = crate::images::upload(
            context.gpu(),
            &self.textures,
            image,
            self.next_texture_id,
        );
        self.gpu_keep.insert(image.slot.clone(), keep);
    }
}

impl RuntimeProgram for NanaBoboProgram {
    type Message = Wake;
    type Error = String;

    fn initialize(
        context: &RuntimeProgramContext<Self::Message>,
    ) -> Result<(Self, Vec<Self::Message>), Self::Error> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        let session = Session::new(runtime.handle().clone())?;
        let dispatch = context.clone();
        session.bind_dispatch(Arc::new(move || dispatch.dispatch(Wake)));
        session.load_account();
        let mut document = RuntimeDocument::new(DocumentId::new(1).expect("document id"));
        let shell = Shell::mount(&mut document, &session).map_err(|error| error.to_string())?;
        Ok((
            Self {
                _runtime: runtime,
                document,
                shell,
                session,
                textures: HostTextureRegistry::new(),
                decoded: HashMap::new(),
                gpu_keep: HashMap::new(),
                next_texture_id: 1,
            },
            vec![Wake],
        ))
    }

    fn with_document<R>(
        &self,
        id: WindowId,
        f: impl FnOnce(&RuntimeDocument) -> R,
    ) -> Result<Option<R>, nana_ui::DocumentAccessError> {
        Ok((id == WindowId::PRIMARY).then(|| f(&self.document)))
    }

    fn with_document_mut<R>(
        &mut self,
        id: WindowId,
        f: impl FnOnce(&mut RuntimeDocument) -> R,
    ) -> Result<Option<R>, nana_ui::DocumentAccessError> {
        Ok((id == WindowId::PRIMARY).then(|| f(&mut self.document)))
    }

    fn update(
        &mut self,
        _message: Self::Message,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        self.apply_pending();
        RuntimeProgramUpdate::redraw(WindowId::PRIMARY)
    }

    fn theme_mode(&self) -> ThemeMode {
        self.session.theme
    }

    fn host_textures(&self, _id: WindowId) -> Option<HostTextureRegistry> {
        Some(self.textures.clone())
    }

    fn prepare_window_frame(
        &mut self,
        _id: WindowId,
        context: &RuntimeProgramContext<Self::Message>,
    ) {
        for image in self.session.take_pending_images() {
            self.upload_image(context, &image);
            self.decoded.insert(image.slot.clone(), image);
        }
    }

    fn rebuild_gpu(&mut self, context: &RuntimeProgramContext<Self::Message>) {
        self.gpu_keep.clear();
        self.textures = HostTextureRegistry::new();
        for image in self.decoded.values().cloned().collect::<Vec<_>>() {
            self.upload_image(context, &image);
        }
    }

    fn window_event(
        &mut self,
        event: WindowEvent,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        match event {
            WindowEvent::CloseRequested { .. } => RuntimeProgramUpdate::exit(),
            _ => RuntimeProgramUpdate::default(),
        }
    }

    fn next_wakeup(&self) -> Option<Instant> {
        self.session.next_wakeup()
    }

    fn wake(
        &mut self,
        now: Instant,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        if self.session.next_wakeup().is_some_and(|deadline| deadline <= now) {
            if self.session.qr.is_some() {
                self.session.inbox.push(AppEvent::PollQr);
            }
            if self.session.room.is_some() {
                self.session.inbox.push(AppEvent::TickStats);
            }
        }
        self.apply_pending();
        RuntimeProgramUpdate::redraw(WindowId::PRIMARY)
    }
}
