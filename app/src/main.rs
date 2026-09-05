// NanaBobo L3 宿主：Rust 控件直接写入 NanaUI RuntimeDocument。

mod images;
#[cfg(all(test, feature = "native-acceptance"))]
mod native_acceptance;
#[cfg(all(test, feature = "native-acceptance"))]
mod native_input_acceptance;
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
use nana_ui_platform::{
    InputEvent, PointerPhase, WindowCommand, WindowEvent, WindowGeometry, WindowId, WindowRole,
};

use crate::images::DecodedImage;
use crate::session::{AppEvent, DesktopEffect, ImageChange, Session, Wake};
use crate::ui::{DesktopDanmakuView, Shell};

#[cfg(all(test, feature = "native-acceptance"))]
fn main() {
    if std::env::args().any(|arg| arg == "--input") {
        native_input_acceptance::run_probe();
    } else {
        native_acceptance::run_probe();
    }
}

#[cfg(not(all(test, feature = "native-acceptance")))]
fn main() -> Result<(), nana_ui::HostedRunError> {
    run_runtime::<NanaBoboProgram>(
        RuntimeWindowSettings::new("Nana播播工具箱")
            .initial_size(1200.0, 800.0)
            .minimum_size(960.0, 600.0)
            .system_caption(false),
    )
}

struct NanaBoboProgram {
    document: RuntimeDocument,
    shell: Shell,
    desktop: Option<DesktopWindow>,
    session: Session,
    textures: HostTextureRegistry,
    decoded: HashMap<String, DecodedImage>,
    gpu_keep: HashMap<String, wgpu::Texture>,
    next_texture_id: u64,
    _runtime: tokio::runtime::Runtime,
}

struct DesktopWindow {
    id: WindowId,
    document: RuntimeDocument,
    view: DesktopDanmakuView,
    passthrough_request: Option<u64>,
}

impl NanaBoboProgram {
    fn apply_pending(&mut self) -> RuntimeProgramUpdate {
        let events = self.session.inbox.drain();
        for event in events {
            self.session.apply(event);
        }
        if let Err(error) = self.shell.sync(&mut self.document, &self.session) {
            eprintln!("Unable to update application view: {error}");
            return RuntimeProgramUpdate::exit();
        }
        let mut update = RuntimeProgramUpdate::redraw_all();
        for effect in self.session.take_desktop_effects() {
            match effect {
                DesktopEffect::Open {
                    generation,
                    settings,
                } => {
                    let id = WindowId(generation);
                    let mut document = RuntimeDocument::new(
                        DocumentId::new(generation + 1).expect("desktop document id"),
                    );
                    match DesktopDanmakuView::mount(&mut document, &self.session) {
                        Ok(view) => {
                            let mut window = RuntimeWindowSettings::new("桌面弹幕")
                                .initial_size(settings.width as f64, settings.height as f64)
                                .minimum_size(280.0, 240.0);
                            window.initial_position =
                                settings.position.map(|p| (p[0] as f64, p[1] as f64));
                            window.transparent = true;
                            window.always_on_top = true;
                            window.focus_on_show = false;
                            window.constrain_to_work_area = true;
                            window.role = WindowRole::Tool;
                            self.desktop = Some(DesktopWindow {
                                id,
                                document,
                                view,
                                passthrough_request: None,
                            });
                            update.window_commands.push(WindowCommand::Open {
                                id,
                                settings: window,
                            });
                        }
                        Err(error) => {
                            eprintln!("Unable to create desktop view: {error}");
                            self.session
                                .inbox
                                .push(AppEvent::DesktopOpenFailed { generation });
                        }
                    }
                }
                DesktopEffect::Close { generation } => {
                    update
                        .window_commands
                        .push(WindowCommand::Close(WindowId(generation)));
                    if self.desktop.as_ref().is_some_and(|w| w.id.0 == generation) {
                        self.desktop = None;
                    }
                }
                DesktopEffect::Focus { generation } => {
                    update
                        .window_commands
                        .push(WindowCommand::Focus(WindowId(generation)));
                }
                DesktopEffect::SetPassthrough {
                    generation,
                    request,
                    enabled,
                } => {
                    if let Some(window) = self.desktop.as_mut().filter(|w| w.id.0 == generation) {
                        window.passthrough_request = Some(request);
                        update
                            .window_commands
                            .push(WindowCommand::SetMousePassthrough {
                                id: window.id,
                                enabled,
                            });
                    }
                }
            }
        }
        if let Some(window) = &mut self.desktop {
            if let Err(error) = window.view.sync(&mut window.document, &self.session) {
                eprintln!("Unable to update desktop view: {error}");
                self.session.inbox.push(AppEvent::CloseDesktopDanmaku);
            }
        }
        update
    }

    fn desktop_geometry(&mut self, id: WindowId, geometry: WindowGeometry) {
        if let Some(window) = self.desktop.as_ref().filter(|w| w.id == id) {
            if let Some(position) = geometry.logical_position {
                self.session.apply(AppEvent::DesktopGeometry {
                    generation: window.id.0,
                    width: geometry.logical_size.0,
                    height: geometry.logical_size.1,
                    position: [position.0, position.1],
                });
            }
        }
    }

    fn upload_image(&mut self, context: &RuntimeProgramContext<Wake>, image: &DecodedImage) {
        self.next_texture_id = self.next_texture_id.saturating_add(1);
        let keep =
            crate::images::upload(context.gpu(), &self.textures, image, self.next_texture_id);
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
        let mut session = Session::new(runtime.handle().clone())?;
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
                desktop: None,
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
        Ok(if id == WindowId::PRIMARY {
            Some(f(&self.document))
        } else {
            self.desktop
                .as_ref()
                .filter(|w| w.id == id)
                .map(|w| f(&w.document))
        })
    }

    fn with_document_mut<R>(
        &mut self,
        id: WindowId,
        f: impl FnOnce(&mut RuntimeDocument) -> R,
    ) -> Result<Option<R>, nana_ui::DocumentAccessError> {
        Ok(if id == WindowId::PRIMARY {
            Some(f(&mut self.document))
        } else {
            self.desktop
                .as_mut()
                .filter(|w| w.id == id)
                .map(|w| f(&mut w.document))
        })
    }

    fn update(
        &mut self,
        _message: Self::Message,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        self.apply_pending()
    }

    fn theme_mode(&self) -> ThemeMode {
        self.session.theme
    }

    fn window_material_mode_for(&self, id: WindowId) -> nana_ui::MaterialEffect {
        use nana_ui::{MaterialEffect, WindowMaterialMode};
        if id != WindowId::PRIMARY {
            return MaterialEffect::Transparent;
        }
        match self.session.appearance.window_material() {
            WindowMaterialMode::Solid => MaterialEffect::Solid,
            WindowMaterialMode::Translucent => MaterialEffect::Transparent,
            WindowMaterialMode::Vibrancy => MaterialEffect::Vibrancy,
            WindowMaterialMode::Mica => MaterialEffect::Mica,
            WindowMaterialMode::Acrylic => MaterialEffect::Acrylic,
        }
    }

    fn appearance_backdrop_opacity_for(&self, id: WindowId) -> f32 {
        if id == WindowId::PRIMARY {
            self.session.appearance.backdrop_opacity()
        } else {
            0.0
        }
    }

    fn host_textures(&self, _id: WindowId) -> Option<HostTextureRegistry> {
        Some(self.textures.clone())
    }

    fn prepare_window_frame(
        &mut self,
        id: WindowId,
        context: &RuntimeProgramContext<Self::Message>,
    ) {
        let needs_layout = if id == WindowId::PRIMARY {
            self.shell.needs_layout_sync(&self.document)
        } else {
            self.desktop
                .as_ref()
                .filter(|w| w.id == id)
                .is_some_and(|w| w.view.needs_layout_sync(&w.document))
        };
        if needs_layout {
            context.dispatch(Wake);
        }
        for change in self.session.take_pending_images() {
            match change {
                ImageChange::Upload(image) => {
                    self.upload_image(context, &image);
                    self.decoded.insert(image.slot.clone(), image);
                }
                ImageChange::Remove(slot) => {
                    self.textures.remove(&slot);
                    self.gpu_keep.remove(&slot);
                    self.decoded.remove(&slot);
                }
            }
        }
    }

    fn rebuild_gpu(&mut self, context: &RuntimeProgramContext<Self::Message>) {
        self.gpu_keep.clear();
        self.textures = HostTextureRegistry::new();
        for image in self.decoded.values().cloned().collect::<Vec<_>>() {
            self.upload_image(context, &image);
        }
    }

    // Scene dispatches native keyboard/IME input before this hook. Unlike the
    // removed Vue delegate, we must not dispatch it again. Control callbacks
    // enqueue Inbox -> Wake -> update; the default accessibility_action uses
    // the same callbacks through the WindowId-routed RuntimeDocument.
    fn input_event(
        &mut self,
        id: WindowId,
        event: &InputEvent,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> Result<RuntimeProgramUpdate, nana_ui::runtime::FrameworkError> {
        if let InputEvent::Pointer {
            phase: PointerPhase::Down,
            button: 0,
            x,
            y,
            ..
        } = event
        {
            let dragging = self
                .desktop
                .as_ref()
                .filter(|w| w.id == id && w.passthrough_request.is_none())
                .filter(|_| self.session.desktop.phase == session::DesktopDanmakuPhase::Adjusting)
                .and_then(|w| w.view.drag_handle_bounds(&w.document))
                .is_some_and(|r| {
                    *x >= r.x && *x < r.x + r.width && *y >= r.y && *y < r.y + r.height
                });
            if dragging {
                return Ok(RuntimeProgramUpdate {
                    window_commands: vec![WindowCommand::Drag(id)],
                    ..RuntimeProgramUpdate::default()
                });
            }
        }
        Ok(RuntimeProgramUpdate::default())
    }

    fn window_event(
        &mut self,
        event: WindowEvent,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        match event {
            WindowEvent::CloseRequested {
                id: WindowId::PRIMARY,
            } => {
                self.session.apply(AppEvent::CloseDesktopDanmaku);
                let mut update = self.apply_pending();
                update.exit = true;
                return update;
            }
            WindowEvent::CloseRequested { id } => {
                if self.desktop.as_ref().is_some_and(|w| w.id == id) {
                    self.session.apply(AppEvent::CloseDesktopDanmaku);
                }
            }
            WindowEvent::Ready { id, geometry } => {
                if let Some(window) = self.desktop.as_ref().filter(|w| w.id == id) {
                    self.session.apply(AppEvent::DesktopOpened {
                        generation: window.id.0,
                    });
                }
                self.desktop_geometry(id, geometry);
            }
            WindowEvent::Resized { id, geometry } | WindowEvent::Moved { id, geometry } => {
                self.desktop_geometry(id, geometry)
            }
            WindowEvent::OpenFailed { id, .. } | WindowEvent::Closed { id } => {
                if let Some(window) = self.desktop.take_if(|w| w.id == id) {
                    self.session
                        .apply(if matches!(event, WindowEvent::OpenFailed { .. }) {
                            AppEvent::DesktopOpenFailed {
                                generation: window.id.0,
                            }
                        } else {
                            AppEvent::DesktopClosed {
                                generation: window.id.0,
                            }
                        });
                }
            }
            WindowEvent::MousePassthroughChanged {
                id,
                enabled,
                result,
            } => {
                if let Some(window) = self.desktop.as_mut().filter(|w| w.id == id) {
                    if let Some(request) = window.passthrough_request.take() {
                        self.session.apply(AppEvent::DesktopPassthroughResult {
                            generation: window.id.0,
                            request,
                            enabled,
                            success: result.is_ok(),
                        });
                    }
                }
            }
            _ => return RuntimeProgramUpdate::default(),
        }
        self.apply_pending()
    }

    fn next_wakeup(&self) -> Option<Instant> {
        self.session.next_wakeup()
    }

    fn wake(
        &mut self,
        now: Instant,
        _context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        self.session.advance(now);
        self.apply_pending()
    }
}
