use std::env;
use std::sync::Arc;
use std::time::Duration;

use egui::Color32;
use egui_wgpu::ScreenDescriptor;
use winit::event::{DeviceEvent, ElementState, KeyEvent, Modifiers, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowBuilder};

use crate::gpu::Gpu;
use crate::gui::Gui;
use crate::mixer::Mixer;
use crate::utils::to_min_sec_millis_str;

#[derive(PartialEq)]
pub enum MixerFocus {
    ChOne,
    ChTwo,
}

pub struct AppVariables {
    pub fps: u8,
    pub frame_counter: u32,
    pub show_debug_panel: bool,
    pub modifiers_key: Modifiers,
    pub mixer: Mixer,
    pub mixer_focus: MixerFocus,
}

pub struct App {
    pub window: Arc<Window>,
    pub gpu: Gpu,
    pub gui: Gui,
    pub app_vars: AppVariables,
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

        let mixer = Mixer::new();

        let app_vars = AppVariables {
            fps: 24,
            frame_counter: 0,
            show_debug_panel: false,
            modifiers_key: Modifiers::default(),
            mixer: mixer,
            mixer_focus: MixerFocus::ChOne,
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
        self.app_vars.modifiers_key = modifiers;

        match self.app_vars.modifiers_key.state() {
            ModifiersState::ALT => {
                self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
            }
            _ => self.window.set_cursor_grab(CursorGrabMode::None).unwrap(),
        }
    }

    pub fn on_key_event(&mut self, physical_key: PhysicalKey, state: ElementState, repeat: bool) {
        match (
            physical_key,
            state,
            repeat,
            self.app_vars.modifiers_key.state(),
        ) {
            (
                PhysicalKey::Code(KeyCode::KeyD),
                ElementState::Pressed,
                false,
                ModifiersState::CONTROL,
            ) => {
                self.app_vars.show_debug_panel = !self.app_vars.show_debug_panel;
            }

            _ => (),
        }
    }

    pub fn on_device_event(&mut self, event: DeviceEvent) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                match (
                    &self.app_vars.mixer_focus,
                    self.app_vars.modifiers_key.state(),
                ) {
                    (MixerFocus::ChOne, ModifiersState::ALT) => {
                        self.app_vars.mixer.soft_touch_one(delta.1 / 1000.0);
                    }
                    (MixerFocus::ChTwo, ModifiersState::ALT) => {
                        self.app_vars.mixer.soft_touch_two(delta.1 / 1000.0);
                    }
                    _ => (),
                }
            }
            _ => (),
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
        if ui
            .add(
                egui::Button::new("ChOne").fill(if app_vars.mixer_focus == MixerFocus::ChOne {
                    Color32::from_rgb(200, 100, 100)
                } else {
                    Color32::from_rgb(100, 100, 100)
                }),
            )
            .clicked()
        {
            app_vars.mixer_focus = MixerFocus::ChOne;
        }

        // Button for ChTwo
        if ui
            .add(
                egui::Button::new("ChTwo").fill(if app_vars.mixer_focus == MixerFocus::ChTwo {
                    Color32::from_rgb(100, 100, 200)
                } else {
                    Color32::from_rgb(100, 100, 100)
                }),
            )
            .clicked()
        {
            app_vars.mixer_focus = MixerFocus::ChTwo;
        }

        let mut cue_one = app_vars.mixer.is_cue_one_enabled();
        ui.toggle_value(&mut cue_one, "Cue ONE");
        app_vars.mixer.set_cue_one(cue_one);

        let mut cue_two = app_vars.mixer.is_cue_two_enabled();
        ui.toggle_value(&mut cue_two, "Cue TWO");
        app_vars.mixer.set_cue_two(cue_two);

        let mut cue_mix = app_vars.mixer.get_cue_mix_value();
        ui.add(egui::Slider::new(&mut cue_mix, 0.0..=1.0).text("Cue Mix"));
        app_vars.mixer.set_cue_mix_value(cue_mix);

        let mut ch_one = app_vars.mixer.get_ch_one_volume();
        ui.add(egui::Slider::new(&mut ch_one, 0.0..=1.0).text("Ch ONE"));
        app_vars.mixer.set_ch_one_volume(ch_one);

        let mut pitch_one = app_vars.mixer.get_pitch_one();
        ui.add(egui::Slider::new(&mut pitch_one, 0.92..=1.08).text("PITCH ONE"));
        app_vars.mixer.set_pitch_one(pitch_one);

        let mut ch_two = app_vars.mixer.get_ch_two_volume();
        ui.add(egui::Slider::new(&mut ch_two, 0.0..=1.0).text("Ch TWO"));
        app_vars.mixer.set_ch_two_volume(ch_two);

        let mut pitch_two = app_vars.mixer.get_pitch_two();
        ui.add(egui::Slider::new(&mut pitch_two, 0.92..=1.08).text("PITCH TWO"));
        app_vars.mixer.set_pitch_two(pitch_two);

        let position_one = app_vars.mixer.get_position_one();
        let duration_one = app_vars.mixer.get_duration_one();
        ui.label(format!(
            "Track One: {} / {}",
            to_min_sec_millis_str(position_one),
            to_min_sec_millis_str(duration_one)
        ));

        let position_two = app_vars.mixer.get_position_two();
        let duration_two = app_vars.mixer.get_duration_two();
        ui.label(format!(
            "Track Two: {} / {}",
            to_min_sec_millis_str(position_two),
            to_min_sec_millis_str(duration_two)
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
