// wgpu context initialisation for WebGPU backend.
//
// The context is stored in a thread-local (WASM is single-threaded) so all
// image-processing functions can access it without passing it around.

use std::cell::RefCell;
use wasm_bindgen::prelude::*;

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

thread_local! {
    static GPU: RefCell<Option<GpuContext>> = const { RefCell::new(None) };
}

/// Run a closure with the GPU context, if one has been initialised.
/// Returns `None` when GPU is unavailable → callers should fall back to CPU.
pub fn with_gpu<R>(f: impl FnOnce(&GpuContext) -> R) -> Option<R> {
    GPU.with(|ctx| ctx.borrow().as_ref().map(f))
}

/// Initialise the wgpu WebGPU context.  Call once from JS after WASM loads.
/// Resolves to `Ok(())` on success; rejects the Promise on failure (no adapter,
/// device request error, etc.) — callers should treat failure as "use CPU path".
#[wasm_bindgen]
pub async fn init_gpu() -> Result<(), JsValue> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| JsValue::from_str("WebGPU: no adapter found"))?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("purikura"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            },
            None,
        )
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    GPU.with(|ctx| {
        *ctx.borrow_mut() = Some(GpuContext { device, queue });
    });

    Ok(())
}
