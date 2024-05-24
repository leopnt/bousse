use std::sync::Arc;

use log::info;
use winit::{dpi::PhysicalSize, window::Window};

pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub samples: u32,
    size: PhysicalSize<u32>,
}

impl Gpu {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                },
                None,
            )
            .await
            .unwrap();

        let window_size = window.inner_size();
        let width = window_size.width.max(1);
        let height = window_size.height.max(1);

        let config = surface
            .get_default_config(&adapter, width, height)
            .expect("Surface isn't supported by the adapter.");

        surface.configure(&device, &config);

        let samples = 1;

        let gpu = Self {
            surface,
            device,
            queue,
            config,
            samples,
            size,
        };

        return gpu;
    }

    pub fn resize(&mut self, physical_size: PhysicalSize<u32>) {
        info!("Surface resize {:?}", physical_size);
        self.size = physical_size;
        self.config.width = physical_size.width;
        self.config.height = physical_size.height;
        self.surface.configure(&self.device, &self.config);
    }
}
