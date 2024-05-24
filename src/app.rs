use std::sync::Arc;
use std::time::Duration;

use egui_wgpu::ScreenDescriptor;
use winit::event::WindowEvent;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::{Window, WindowBuilder};

use crate::gpu::Gpu;
use crate::gui::Gui;

pub struct App {
    pub window: Arc<Window>,
    pub gpu: Gpu,
    pub gui: Gui,
    pub button_click_counter: u8,
    pub frame_counter: u32,
    pub fps: u8,
    show_debug_panel: bool,
}

impl App {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let window = WindowBuilder::new()
            .with_title(format!(
                "{} v{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .build(&event_loop)
            .unwrap();
        let window = Arc::new(window);

        let gpu = pollster::block_on(Gpu::new(Arc::clone(&window)));

        let gui = Gui::new(&window, &gpu);

        Self {
            window: window,
            gpu: gpu,
            gui: gui,
            button_click_counter: 0,
            frame_counter: 0,
            fps: 24,
            show_debug_panel: true,
        }
    }

    pub fn on_window_event(&mut self, event: WindowEvent, elwt: &EventLoopWindowTarget<()>) {
        self.gui.handle_event(&self.window, &event);

        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                elwt.exit();
            }

            WindowEvent::RedrawRequested => {
                self.frame_counter += 1;

                let surface_texture = self
                    .gpu
                    .surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");

                let surface_view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = self
                    .gpu
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let screen_descriptor = ScreenDescriptor {
                    size_in_pixels: [self.gpu.config.width, self.gpu.config.height],
                    pixels_per_point: self.window.scale_factor() as f32,
                };

                self.gui.draw(
                    &self.gpu.device,
                    &self.gpu.queue,
                    &mut encoder,
                    &self.window,
                    &surface_view,
                    screen_descriptor,
                    |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            ui.label("Hello World!");
                            if ui.button("Click me").clicked() {
                                self.button_click_counter += 1;
                            }
                            ui.label(format!(
                                "Thanks for clicking {}x ❤",
                                self.button_click_counter
                            ));
                        });

                        if self.show_debug_panel {
                            egui::TopBottomPanel::bottom("debug_panel").show(ui, |ui| {
                                ui.label("Debug Panel");
                                ui.separator();
                                ui.label(format!("frame_counter: {}", self.frame_counter));
                                ui.label(format!("window_size: {:?}", self.window.inner_size()));
                            });
                        }
                    },
                );

                self.gpu.queue.submit(Some(encoder.finish()));
                surface_texture.present();
            }
            WindowEvent::Resized(physical_size) => {
                self.gpu.resize(physical_size);
            }
            _ => (),
        }
    }

    pub fn on_resume_time_reached(&self, elwt: &EventLoopWindowTarget<()>) {
        elwt.set_control_flow(ControlFlow::wait_duration(Duration::from_millis(
            (1000 as f32 / self.fps as f32) as u64,
        )));
        self.window.request_redraw();
    }
}
