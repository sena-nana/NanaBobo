//! NanaBobo 原生宿主:V8 引擎执行 Vue IIFE,经 NanaUI Scene 画进 winit 窗口。

mod host_api;

use std::time::Instant;

use nana_js_engine::{JsEngine, JsEngineError, RuntimeArtifact};
use nana_ui::{
    HostTextureRegistry, HostedGpuResources, RuntimeProgram, RuntimeProgramContext,
    RuntimeProgramUpdate, RuntimeWindowSettings, run_runtime,
};
use nana_ui_platform::{InputEvent, WindowEvent, WindowId};
use nana_ui_scene::RuntimeDocument;
use nana_ui_vue::{BridgeEvent, VueHostedRuntime, VueRuntimeProgram};

use crate::host_api::registry as host_registry;

const APP_JS: &str = include_str!("../../ui/dist/nanabobo.iife.js");
const APP_CSS: &str = include_str!("../../ui/dist/nanabobo-ui.css");

type AppEngine = nana_js_v8::V8Engine;

fn main() -> Result<(), nana_ui::HostedRunError> {
    run_runtime::<NanaBoboProgram>(
        RuntimeWindowSettings::new("Nana播播工具箱")
            .initial_size(1200.0, 800.0)
            .minimum_size(960.0, 600.0)
            .system_caption(true),
    )
}

fn artifact() -> RuntimeArtifact {
    RuntimeArtifact::from_source("nanabobo.js", APP_JS)
}

fn build_runtime(
    gpu: HostedGpuResources,
    width: u32,
    height: u32,
    scale_factor: f32,
) -> Result<VueHostedRuntime<AppEngine>, JsEngineError> {
    let engine = AppEngine::new();
    let events = engine
        .host_event_sender()
        .expect("v8 engine exposes a host event sender");
    let mut runtime = VueHostedRuntime::new(
        engine,
        artifact(),
        host_registry(events),
        width,
        height,
        scale_factor,
    )?;
    runtime.bind_host_gpu(gpu)?;
    runtime.inject_stylesheet(APP_CSS)?;
    let mount = runtime.engine_mut().resolve_function("__nanabobo.mount")?;
    runtime.engine_mut().invoke(mount, &[])?;
    runtime.engine_mut().run_microtasks()?;
    Ok(runtime)
}

struct NanaBoboProgram {
    inner: VueRuntimeProgram<AppEngine>,
}

impl RuntimeProgram for NanaBoboProgram {
    type Message = BridgeEvent;
    type Error = JsEngineError;

    fn initialize(
        context: &RuntimeProgramContext<Self::Message>,
    ) -> Result<(Self, Vec<Self::Message>), Self::Error> {
        let geometry = context.geometry();
        let runtime = build_runtime(
            context.gpu().clone(),
            geometry.physical_size.0.max(1),
            geometry.physical_size.1.max(1),
            geometry.scale_factor.max(0.01),
        )?;
        Ok((
            Self {
                inner: VueRuntimeProgram::from_runtime(runtime),
            },
            Vec::new(),
        ))
    }

    fn document(&self, id: WindowId) -> Option<&RuntimeDocument> {
        self.inner.document(id)
    }

    fn document_mut(&mut self, id: WindowId) -> Option<&mut RuntimeDocument> {
        self.inner.document_mut(id)
    }

    fn update(
        &mut self,
        message: Self::Message,
        context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        self.inner.update(message, context)
    }

    fn theme_mode(&self) -> nana_ui::ThemeMode {
        self.inner.theme_mode()
    }

    fn host_textures(&self, id: WindowId) -> Option<HostTextureRegistry> {
        self.inner.host_textures(id)
    }

    fn prepare_window_frame(
        &mut self,
        id: WindowId,
        context: &RuntimeProgramContext<Self::Message>,
    ) {
        self.inner.prepare_window_frame(id, context);
    }

    fn take_accessibility_update(
        &mut self,
        id: WindowId,
    ) -> Option<nana_ui_runtime::AccessibilityUpdate> {
        self.inner.take_accessibility_update(id)
    }

    fn rebuild_gpu(&mut self, context: &RuntimeProgramContext<Self::Message>) {
        self.inner.rebuild_gpu(context);
    }

    fn input_event(
        &mut self,
        id: WindowId,
        event: &InputEvent,
        context: &RuntimeProgramContext<Self::Message>,
    ) -> Result<RuntimeProgramUpdate, nana_ui_runtime::FrameworkError> {
        self.inner.input_event(id, event, context)
    }

    fn window_event(
        &mut self,
        event: WindowEvent,
        context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        self.inner.window_event(event, context)
    }

    fn next_wakeup(&self) -> Option<Instant> {
        self.inner.next_wakeup()
    }

    fn wake(
        &mut self,
        now: Instant,
        context: &RuntimeProgramContext<Self::Message>,
    ) -> RuntimeProgramUpdate {
        self.inner.wake(now, context)
    }

    fn sync_animation_clock(&mut self, epoch: Instant) {
        self.inner.sync_animation_clock(epoch);
    }

    fn animation_frame(
        &mut self,
        id: WindowId,
        frame: nana_ui_runtime::AnimationFrame,
        context: &RuntimeProgramContext<Self::Message>,
    ) -> Result<RuntimeProgramUpdate, nana_ui_runtime::FrameworkError> {
        self.inner.animation_frame(id, frame, context)
    }

    fn accessibility_action(
        &mut self,
        id: WindowId,
        request: nana_ui::AccessibilityActionRequest,
        context: &RuntimeProgramContext<Self::Message>,
    ) -> Result<RuntimeProgramUpdate, nana_ui_runtime::FrameworkError> {
        self.inner.accessibility_action(id, request, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nana_ui::HostedGpuResources;
    use nana_ui_vue::{VueWindowId, WidgetKind};
    use std::sync::Arc;

    fn gpu() -> HostedGpuResources {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::from_env().unwrap_or_default(),
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .expect("无头 WGPU 适配器");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("nanabobo spike test"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            }))
            .expect("无头 WGPU 设备");
        HostedGpuResources::from_existing(adapter, Arc::new(device), Arc::new(queue))
    }

    fn snapshot_labels(runtime: &mut VueHostedRuntime<AppEngine>) -> Vec<String> {
        for _ in 0..24 {
            runtime.pump().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let host = runtime.vue().host(VueWindowId::PRIMARY).unwrap();
        let snapshot = host.lock().unwrap().semantic_snapshot();
        snapshot
            .widgets
            .iter()
            .map(|widget| widget.props.label.clone())
            .collect()
    }

    fn wait_for_label(runtime: &mut VueHostedRuntime<AppEngine>, needle: &str) -> bool {
        let mut last = Vec::new();
        for _ in 0..40 {
            last = snapshot_labels(runtime);
            if last.iter().any(|l| l.contains(needle)) {
                return true;
            }
        }
        eprintln!("LABELS for {needle:?}: {last:?}");
        false
    }

    #[test]
    fn app_shell_mounts_with_navigation_and_home_page() {
        let gpu = gpu();
        let mut runtime = build_runtime(gpu, 1200, 800, 1.0).unwrap();

        let labels = snapshot_labels(&mut runtime);
        for expected in ["首页", "主播助手", "数据统计", "历史记录", "设置"] {
            assert!(
                labels.iter().any(|l| l.contains(expected)),
                "侧边导航缺少「{expected}」: {labels:?}"
            );
        }

        // 主页内容(工具区标题 + 房间连接表单)。会话状态依赖真实凭据与
        // 网络,这里只断言与登录态无关的稳定内容。
        let ready = wait_for_label(&mut runtime, "实时直播工具");
        eprintln!("probe_last={}", crate::host_api::probe_last());
        assert!(ready, "主页工具区未渲染");
        assert!(
            wait_for_label(&mut runtime, "连接直播间"),
            "房间连接面板未渲染"
        );
    }
}
