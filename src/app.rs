use std::sync::Arc;
use std::time::Duration;

use egui_wgpu::ScreenDescriptor;
use winit::event::{ElementState, KeyEvent, Modifiers, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowBuilder};

use crate::gpu::Gpu;
use crate::gui::Gui;

struct AppVariables {
    pub fps: u8,
    pub frame_counter: u32,
    pub button_click_counter: u8,
    pub show_debug_panel: bool,
}

pub struct App {
    pub window: Arc<Window>,
    pub gpu: Gpu,
    pub gui: Gui,
    app_vars: AppVariables,
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

        let app_vars = AppVariables {
            fps: 24,
            frame_counter: 0,
            button_click_counter: 0,
            show_debug_panel: false,
        };

        Self {
            window: window,
            gpu: gpu,
            gui: gui,
            app_vars: app_vars,
        }
    }

    fn surface_texture(&self) -> wgpu::SurfaceTexture {
        self.gpu
            .surface
            .get_current_texture()
            .expect("Failed to acquire next swap chain texture")
    }

    fn surface_view(&self, surface_texture: &wgpu::SurfaceTexture) -> wgpu::TextureView {
        surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn encoder(&self) -> wgpu::CommandEncoder {
        self.gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }

    fn screen_descriptor(&self) -> ScreenDescriptor {
        ScreenDescriptor {
            size_in_pixels: [self.gpu.config.width, self.gpu.config.height],
            pixels_per_point: self.window.scale_factor() as f32,
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
                self.app_vars.frame_counter += 1;

                let mut encoder = self.encoder();
                let surface_texture = self.surface_texture();
                let surface_view = self.surface_view(&surface_texture);

                self.gui.draw(
                    &self.gpu.device,
                    &self.gpu.queue,
                    &mut encoder,
                    &self.window,
                    &surface_view,
                    self.screen_descriptor(),
                    |ctx| run_ui(ctx, &self.window, &mut self.app_vars),
                );

                self.gpu.queue.submit(Some(encoder.finish()));
                surface_texture.present();
            }
            WindowEvent::Resized(physical_size) => {
                self.gpu.resize(physical_size);
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.on_modifiers_key_changed(modifiers);
            }

            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        repeat,
                        ..
                    },
                ..
            } => {
                self.on_key_event(physical_key, state, repeat);
            }

            _ => (),
        }
    }

    pub fn on_modifiers_key_changed(&mut self, modifiers: Modifiers) {
        println!("{:?}", modifiers);
    }

    pub fn on_key_event(&mut self, physical_key: PhysicalKey, state: ElementState, repeat: bool) {
        match (physical_key, state, repeat) {
            (PhysicalKey::Code(KeyCode::KeyD), ElementState::Pressed, false) => {
                self.app_vars.show_debug_panel = !self.app_vars.show_debug_panel;
            },

            _ => ()
        }
    }

    pub fn on_resume_time_reached(&self, elwt: &EventLoopWindowTarget<()>) {
        elwt.set_control_flow(ControlFlow::wait_duration(Duration::from_millis(
            (1000 as f32 / self.app_vars.fps as f32) as u64,
        )));
        self.window.request_redraw();
    }
}

fn run_ui(ctx: &egui::Context, window: &Arc<Window>, app_vars: &mut AppVariables) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.label("Hello World!");
        if ui.button("Click me").clicked() {
            app_vars.button_click_counter += 1;
        }
        ui.label(format!(
            "Thanks for clicking {}x ❤",
            app_vars.button_click_counter
        ));
    });

    if app_vars.show_debug_panel {
        egui::TopBottomPanel::bottom("debug_panel").show(ctx, |ui| {
            ui.label("Debug Panel");
            ui.separator();
            ui.label(format!("frame_counter: {}", app_vars.frame_counter));
            ui.label(format!("window_size: {:?}", window.inner_size()));
        });
    }
}
