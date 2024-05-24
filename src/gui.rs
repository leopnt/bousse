use egui::Context;
use egui_wgpu::{Renderer, ScreenDescriptor};
use egui_winit::State;
use wgpu::{CommandEncoder, Device, Queue, TextureView};
use winit::{event::WindowEvent, window::Window};

use crate::gpu::Gpu;

pub struct Gui {
    state: State,
    renderer: Renderer,
    pixels_per_point: f32,
}

impl Gui {
    pub fn new(window: &Window, gpu_state: &Gpu) -> Self {
        let egui_context = Context::default();
        let id = egui_context.viewport_id();

        let egui_state = State::new(egui_context.clone(), id, &window, None, None);

        let egui_renderer = Renderer::new(
            &gpu_state.device,
            gpu_state.config.format,
            None,
            gpu_state.samples,
        );

        Self {
            state: egui_state,
            renderer: egui_renderer,
            pixels_per_point: window.scale_factor() as f32,
        }
    }

    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }

    pub fn draw(
        &mut self,
        device: &Device,
        queue: &Queue,
        encoder: &mut CommandEncoder,
        window: &Window,
        window_surface_view: &TextureView,
        screen_descriptor: ScreenDescriptor,
        run_ui: impl FnOnce(&Context),
    ) {
        let raw_input = self.state.take_egui_input(&window);
        let full_output = self.state.egui_ctx().run(raw_input, |_ui| {
            run_ui(&self.state.egui_ctx());
        });

        self.state
            .handle_platform_output(&window, full_output.platform_output);

        let tris = self
            .state
            .egui_ctx()
            .tessellate(full_output.shapes, self.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(&device, &queue, *id, &image_delta);
        }
        self.renderer
            .update_buffers(&device, &queue, encoder, &tris, &screen_descriptor);
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &window_surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("Egui render pass"),
            occlusion_query_set: None,
        });
        self.renderer.render(&mut rpass, &tris, &screen_descriptor);
        drop(rpass);
        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }
    }
}
