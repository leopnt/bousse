use std::env;
use std::sync::Arc;
use std::time::Duration;

use egui::{Rounding, Visuals};
use egui_wgpu::ScreenDescriptor;
use winit::event::{DeviceEvent, ElementState, KeyEvent, Modifiers, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowBuilder};

use crate::gpu::Gpu;
use crate::gui::Gui;
use crate::mixer::{ChControl, Mixer};
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

        match self.app_vars.mixer_focus {
            MixerFocus::ChOne => {
                match self.app_vars.modifiers_key.state().bits() {
                    0x100 => {
                        self.app_vars
                            .mixer
                            .set_ch_one_control_state(ChControl::SoftTouching);
                        self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                    } // ALT
                    0x800 => {
                        self.app_vars
                            .mixer
                            .set_ch_one_control_state(ChControl::Seeking);
                        self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                    } // SUPER
                    0x900 => {
                        self.app_vars
                            .mixer
                            .set_ch_one_control_state(ChControl::Cueing);
                        self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                    } // ALT | SUPER
                    0x0 => {
                        self.app_vars
                            .mixer
                            .set_ch_one_control_state(ChControl::Untouched);
                        self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                    }
                    _ => (),
                }
            }

            MixerFocus::ChTwo => {
                match self.app_vars.modifiers_key.state().bits() {
                    0x100 => {
                        self.app_vars
                            .mixer
                            .set_ch_two_control_state(ChControl::SoftTouching);
                        self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                    } // ALT
                    0x800 => {
                        self.app_vars
                            .mixer
                            .set_ch_two_control_state(ChControl::Seeking);
                        self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                    } // SUPER
                    0x900 => {
                        self.app_vars
                            .mixer
                            .set_ch_two_control_state(ChControl::Cueing);
                        self.window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                    } // ALT | SUPER
                    0x0 => {
                        self.app_vars
                            .mixer
                            .set_ch_two_control_state(ChControl::Untouched);
                        self.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                    }
                    _ => (),
                }
            }
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
            DeviceEvent::MouseMotion { delta } => match &self.app_vars.mixer_focus {
                MixerFocus::ChOne => {
                    self.app_vars.mixer.touch_one(delta.1 / 1000.0);
                }
                MixerFocus::ChTwo => {
                    self.app_vars.mixer.touch_two(delta.1 / 1000.0);
                }
            },
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
    let mut theme_visuals = Visuals::light();
    theme_visuals.extreme_bg_color = theme_visuals.widgets.inactive.weak_bg_fill;
    ctx.set_visuals(theme_visuals.clone());

    let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
    if !dropped_files.is_empty() {
        match app_vars.mixer_focus {
            MixerFocus::ChOne => app_vars.mixer.load_ch_one(
                dropped_files[0]
                    .path
                    .as_ref()
                    .expect("Cannot get file path from drag and drop"),
            ),
            MixerFocus::ChTwo => app_vars.mixer.load_ch_two(
                dropped_files[0]
                    .path
                    .as_ref()
                    .expect("Cannot get file path from drag and drop"),
            ),
        }
    }

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.label("Top Panel");
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let mut cue_mix = app_vars.mixer.get_cue_mix_value();
        ui.add(egui::Slider::new(&mut cue_mix, 0.0..=1.0).text("Cue Mix"));
        app_vars.mixer.set_cue_mix_value(cue_mix);

        ui.separator();

        ui.columns(2, |cols| {
            cols[0].vertical_centered_justified(|ui| {
                let position_one = app_vars.mixer.get_position_one();
                let duration_one = app_vars.mixer.get_duration_one();
                let pitch_one = app_vars.mixer.get_pitch_one();
                ui.add(
                    egui::ProgressBar::new((position_one / duration_one) as f32)
                        .text(format!(
                            "{} / {}",
                            to_min_sec_millis_str(position_one),
                            to_min_sec_millis_str(duration_one / pitch_one)
                        ))
                        .rounding(Rounding::default()),
                );

                ui.horizontal(|ui| {
                    let mut ch_one = app_vars.mixer.get_ch_one_volume();
                    ui.add(
                        egui::Slider::new(&mut ch_one, 0.0..=1.0)
                            .text("Ch ONE")
                            .vertical(),
                    );
                    app_vars.mixer.set_ch_one_volume(ch_one);

                    let mut pitch_one = app_vars.mixer.get_pitch_one_target();
                    ui.add(
                        egui::Slider::new(&mut pitch_one, 1.08..=0.92)
                            .text("PITCH ONE")
                            .vertical(),
                    );
                    app_vars.mixer.set_pitch_one_target(pitch_one);
                });

                let mut cue_one = app_vars.mixer.is_cue_one_enabled();
                if ui
                    .add(egui::Button::new("Cue").fill(if cue_one {
                        egui::Color32::LIGHT_BLUE
                    } else {
                        theme_visuals.widgets.inactive.weak_bg_fill
                    }))
                    .clicked()
                {
                    cue_one = !cue_one;
                }
                app_vars.mixer.set_cue_one(cue_one);

                if ui
                    .add(egui::Button::new("Focus ChOne").fill(
                        if app_vars.mixer_focus == MixerFocus::ChOne {
                            egui::Color32::from_rgb(170, 170, 255)
                        } else {
                            theme_visuals.widgets.inactive.weak_bg_fill
                        },
                    ))
                    .clicked()
                {
                    app_vars.mixer_focus = MixerFocus::ChOne;
                }

                if ui.add(egui::Button::new("START-STOP")).clicked() {
                    app_vars.mixer.toggle_start_stop_one();
                }
            });

            cols[1].vertical_centered_justified(|ui| {
                let position_two = app_vars.mixer.get_position_two();
                let duration_two = app_vars.mixer.get_duration_two();
                let pitch_two = app_vars.mixer.get_pitch_two();
                ui.add(
                    egui::ProgressBar::new((position_two / duration_two) as f32)
                        .text(format!(
                            "{} / {}",
                            to_min_sec_millis_str(position_two),
                            to_min_sec_millis_str(duration_two / pitch_two)
                        ))
                        .rounding(Rounding::default()),
                );

                ui.horizontal(|ui| {
                    let mut ch_two = app_vars.mixer.get_ch_two_volume();
                    ui.add(
                        egui::Slider::new(&mut ch_two, 0.0..=1.0)
                            .text("Ch TWO")
                            .vertical(),
                    );
                    app_vars.mixer.set_ch_two_volume(ch_two);

                    let mut pitch_two = app_vars.mixer.get_pitch_two_target();
                    ui.add(
                        egui::Slider::new(&mut pitch_two, 1.08..=0.92)
                            .text("PITCH TWO")
                            .vertical(),
                    );
                    app_vars.mixer.set_pitch_two_target(pitch_two);
                });

                let mut cue_two = app_vars.mixer.is_cue_two_enabled();
                if ui
                    .add(egui::Button::new("Cue").fill(if cue_two {
                        egui::Color32::LIGHT_BLUE
                    } else {
                        theme_visuals.widgets.inactive.weak_bg_fill
                    }))
                    .clicked()
                {
                    cue_two = !cue_two;
                }
                app_vars.mixer.set_cue_two(cue_two);

                if ui
                    .add(egui::Button::new("Focus ChTwo").fill(
                        if app_vars.mixer_focus == MixerFocus::ChTwo {
                            egui::Color32::from_rgb(170, 170, 255)
                        } else {
                            theme_visuals.widgets.inactive.weak_bg_fill
                        },
                    ))
                    .clicked()
                {
                    app_vars.mixer_focus = MixerFocus::ChTwo;
                }

                if ui.add(egui::Button::new("START-STOP")).clicked() {
                    app_vars.mixer.toggle_start_stop_two();
                }
            });
        });
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
