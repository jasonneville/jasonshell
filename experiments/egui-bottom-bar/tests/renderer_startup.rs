//! Backend regression + explicitly requested real GPU startup integration.
//! No alternate wgpu dependency/features: use the exact renderer eframe reexports.
#![cfg(target_os = "windows")]

use eframe::wgpu;
use std::{
    future::Future,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

fn windows_backends() -> wgpu::Backends {
    wgpu::Backends::DX12 | wgpu::Backends::VULKAN | wgpu::Backends::GL
}

#[test]
fn windows_eframe_renderer_has_implemented_backend_enabled() {
    let enabled = wgpu::Instance::enabled_backend_features();
    assert!(
        enabled.intersects(windows_backends()),
        "eframe's actual wgpu build must enable a Windows backend (DX12/VULKAN/GL); enabled={enabled:?}"
    );
}

struct ThreadWake(thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

// No new executor dependency or features that could accidentally repair RED.
fn await_bounded<T>(future: impl Future<Output = T>, stage: &str) -> T {
    let mut future = std::pin::pin!(future);
    let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
    let mut cx = Context::from_waker(&waker);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => {
                assert!(
                    Instant::now() < deadline,
                    "renderer smoke timed out: {stage}"
                );
                thread::park_timeout(Duration::from_millis(10));
            }
        }
    }
}

#[test]
#[ignore = "explicit GPU/driver smoke; run --test renderer_startup -- --ignored --nocapture with external process timeout"]
fn real_windows_instance_adapter_device_and_command_submission_start() {
    // Deliberately don't precheck flags: expose the real Instance::new panic too.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: windows_backends(),
        ..Default::default()
    });
    let adapter = await_bounded(
        instance.request_adapter(&wgpu::RequestAdapterOptions::default()),
        "adapter",
    )
    .expect("real Windows adapter required; adapter/driver failure is not a pass");
    let info = adapter.get_info();
    assert!(matches!(
        info.backend,
        wgpu::Backend::Dx12 | wgpu::Backend::Vulkan | wgpu::Backend::Gl
    ));
    println!("renderer adapter: {info:?}");
    let (device, queue) = await_bounded(
        adapter.request_device(&wgpu::DeviceDescriptor::default()),
        "device",
    )
    .expect("real GPU device initialization must succeed");
    let encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("safe fixture renderer startup smoke"),
    });
    queue.submit([encoder.finish()]);
    // Submit is exercised, not a claim of rendered/presented pixels or completion.
    println!("renderer smoke: instance + adapter + device + command submission ready");
}
