use std::sync::Arc;
use std::time::{Duration, Instant};

use egui::{Image, Label, Layout, Rounding, ScrollArea, SelectableLabel, Visuals};
use egui_wgpu::ScreenDescriptor;
use winit::event::{DeviceEvent, ElementState, KeyEvent, Modifiers, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowBuilder};

use crate::controller::{BoothEvent, Controller, TurntableFocus};
use crate::cover_img::CoverImg;
use crate::file_navigator::FileNavigator;
use crate::gpu::Gpu;
use crate::gui::Gui;
use crate::mixer::Mixer;
use crate::processable::Processable;
use crate::turntable::Turntable;
use crate::utils::{remap, to_min_sec_millis_str};

pub struct AppData {
    pub fps: u8,
    pub frame_counter: u32,
    pub show_debug_panel: bool,
    pub mixer: Mixer,
    pub turntable_one: Turntable,
    pub turntable_two: Turntable,
    pub turntable_focus: TurntableFocus,
    pub modifiers_key: Modifiers,
    pub file_navigator: FileNavigator,
    pub cover_one: CoverImg,
    pub cover_two: CoverImg,
}

pub struct App {
    pub window: Arc<Window>,
    pub gpu: Gpu,
    pub gui: Gui,
    pub app_data: AppData,
    pub controller: Controller,
    pub delta_timer: Instant,
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
        let audio_manager_clone_one = mixer.get_audio_manager();
        let audio_manager_clone_two = mixer.get_audio_manager();
        let ch_one_track_clone = mixer.get_ch_one_track();
        let ch_two_track_clone = mixer.get_ch_two_track();

        let app_data = AppData {
            fps: 24,
            frame_counter: 0,
            show_debug_panel: true,
            mixer: mixer,
            turntable_one: Turntable::new(audio_manager_clone_one, ch_one_track_clone),
            turntable_two: Turntable::new(audio_manager_clone_two, ch_two_track_clone),
            turntable_focus: TurntableFocus::One,
            modifiers_key: Modifiers::default(),
            file_navigator: FileNavigator::new(
                &dotenv::var("ROOT_DIR").expect("ROOT_DIR environment variable not present"),
            ),
            cover_one: CoverImg::default(),
            cover_two: CoverImg::default(),
        };

        Self {
            window: window,
            gpu: gpu,
            gui: gui,
            app_data: app_data,
            controller: Controller::new(),
            delta_timer: Instant::now(),
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
                self.app_data.frame_counter += 1;

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
                    |ctx| run_ui(ctx, &self.window, &mut self.app_data, &mut self.controller),
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
        self.app_data.modifiers_key = modifiers;

        match modifiers.state() {
            ModifiersState::SUPER => self
                .controller
                .handle_event(&mut self.app_data, BoothEvent::ScratchBegin),
            _ => self
                .controller
                .handle_event(&mut self.app_data, BoothEvent::ScratchEnd),
        }

        match modifiers.state() {
            ModifiersState::ALT | ModifiersState::SUPER => self
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::Locked)
                .unwrap(),
            _ => self
                .window
                .set_cursor_grab(winit::window::CursorGrabMode::None)
                .unwrap(),
        };
    }

    pub fn on_key_event(&mut self, physical_key: PhysicalKey, state: ElementState, repeat: bool) {
        match (
            physical_key,
            state,
            repeat,
            self.app_data.modifiers_key.state(),
        ) {
            (
                PhysicalKey::Code(KeyCode::KeyD),
                ElementState::Pressed,
                false,
                ModifiersState::CONTROL,
            ) => {
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::ToggleDebug);
            }
            (PhysicalKey::Code(KeyCode::ArrowDown), ElementState::Pressed, _, _) => {
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::FileNavigatorDown);
            }
            (PhysicalKey::Code(KeyCode::ArrowUp), ElementState::Pressed, _, _) => {
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::FileNavigatorUp);
            }
            (PhysicalKey::Code(KeyCode::ArrowRight), ElementState::Pressed, false, _) => {
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::FileNavigatorSelect);
            }
            (PhysicalKey::Code(KeyCode::ArrowLeft), ElementState::Pressed, false, _) => {
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::FileNavigatorBack);
            }
            (PhysicalKey::Code(KeyCode::KeyD), ElementState::Released, false, _) => {
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::ToggleStartStopOne);
            }
            (PhysicalKey::Code(KeyCode::KeyF), ElementState::Released, false, _) => {
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::ToggleStartStopTwo);
            }
            _ => (),
        }
    }

    pub fn on_device_event(&mut self, event: DeviceEvent) {
        match (event, self.app_data.modifiers_key.state()) {
            (DeviceEvent::MouseMotion { delta }, ModifiersState::ALT | ModifiersState::SUPER) => {
                let dir = delta.1.signum();
                let mag = delta.1.abs().powf(0.65); // apply pow to compensate for mouse acceleration / non linearity

                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::ForceApplied(-dir * mag));
            }
            _ => (),
        }
    }

    pub fn on_resume_time_reached(&mut self, elwt: &EventLoopWindowTarget<()>) {
        self.process(self.delta_timer.elapsed().as_secs_f64());
        self.delta_timer = Instant::now();

        elwt.set_control_flow(ControlFlow::wait_duration(Duration::from_millis(
            (1000 as f32 / self.app_data.fps as f32) as u64,
        )));
        self.window.request_redraw();
    }

    pub fn on_midi_event(&mut self, message: &[u8]) {
        // hard coded values for my controller here
        match message {
            [144, 1, _] => self
                .controller
                .handle_event(&mut self.app_data, BoothEvent::ToggleCueOne),
            [144, 4, _] => self
                .controller
                .handle_event(&mut self.app_data, BoothEvent::ToggleCueTwo),
            [144, 3, _] => self.controller.handle_event(
                &mut self.app_data,
                BoothEvent::FocusChanged(TurntableFocus::One),
            ),
            [144, 6, _] => self.controller.handle_event(
                &mut self.app_data,
                BoothEvent::FocusChanged(TurntableFocus::Two),
            ),
            [_, 18, value] => {
                let value = remap(*value as f64, 0.0, 127.0, 0.0, 1.0);
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::VolumeOneChanged(value))
            }
            [_, 22, value] => {
                let value = remap(*value as f64, 0.0, 127.0, 0.0, 1.0);
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::VolumeTwoChanged(value))
            }
            [_, 19, value] => {
                let value = remap(*value as f64, 0.0, 127.0, 1.06, 0.94);
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::PitchOneChanged(value))
            }
            [_, 23, value] => {
                let value = remap(*value as f64, 0.0, 127.0, 1.06, 0.94);
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::PitchTwoChanged(value))
            }
            [_, 17, value] => {
                let value = remap(
                    ((*value + 1) as f64).log10() as f64,
                    0.0,
                    127.0_f64.log10(),
                    -24.0,
                    3.0,
                );
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::EqLowOneChanged(value))
            }
            [_, 16, value] => {
                let value = remap(
                    ((*value + 1) as f64).log10() as f64,
                    0.0,
                    127.0_f64.log10(),
                    -24.0,
                    3.0,
                );
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::EqHighOneChanged(value))
            }
            [_, 21, value] => {
                let value = remap(
                    ((*value + 1) as f64).log10() as f64,
                    0.0,
                    127.0_f64.log10(),
                    -24.0,
                    3.0,
                );
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::EqLowTwoChanged(value))
            }
            [_, 20, value] => {
                let value = remap(
                    ((*value + 1) as f64).log10() as f64,
                    0.0,
                    127.0_f64.log10(),
                    -24.0,
                    3.0,
                );
                self.controller
                    .handle_event(&mut self.app_data, BoothEvent::EqHighTwoChanged(value))
            }
            _ => {
                log::info!("App received unmatched midi message: {:?}", message);
            }
        }
    }
}

impl Processable for App {
    fn process(&mut self, delta: f64) {
        self.app_data.turntable_one.process(delta);
        self.app_data.turntable_two.process(delta);
    }
}

fn run_ui(
    ctx: &egui::Context,
    window: &Arc<Window>,
    app_data: &mut AppData,
    controller: &mut Controller,
) {
    let mut theme_visuals = Visuals::light();
    theme_visuals.extreme_bg_color = theme_visuals.widgets.inactive.weak_bg_fill;
    ctx.set_visuals(theme_visuals.clone());

    let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
    if !dropped_files.is_empty() {
        let path = dropped_files[0]
            .path
            .as_ref()
            .expect("Cannot get file path from drag and drop");
        controller.handle_event(app_data, BoothEvent::TrackLoad(path));
    }

    egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
        ui.label("Top Panel");
    });

    egui::CentralPanel::default().show(ctx, |ui| {
        let mut cue_mix = app_data.mixer.get_cue_mix_value();
        ui.add(egui::Slider::new(&mut cue_mix, 0.0..=1.0).text("Cue Mix"));
        controller.handle_event(app_data, BoothEvent::CueMixChanged(cue_mix));

        ui.separator();

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height() * 0.3)
            .show(ui, |ui| {
                if app_data.file_navigator.entries().is_empty() {
                    ui.add(Label::new("Oops! There is nothing here..."));
                    return;
                };

                ui.with_layout(Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    for entry in app_data.file_navigator.entries().clone().iter() {
                        ui.add(SelectableLabel::new(
                            app_data.file_navigator.selected() == Some(entry),
                            entry,
                        ));

                        // ensure the selected element is visible
                        if app_data.file_navigator.selected() == Some(entry) {
                            ui.scroll_to_cursor(Some(egui::Align::Center));
                        }
                    }
                });
            });

        ui.separator();

        ui.columns(2, |cols| {
            cols[0].vertical_centered_justified(|ui| {
                ui.with_layout(Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.add(match app_data.turntable_one.currently_loaded() {
                        Some(path) => Label::new(path.split('/').last().unwrap()),
                        None => Label::new("No Track Loaded"),
                    })
                });

                let (position, duration, position_display, duration_display) = match (
                    app_data.turntable_one.position(),
                    app_data.turntable_one.duration(),
                ) {
                    (Some(position), Some(duration)) => (
                        position,
                        duration,
                        to_min_sec_millis_str(position),
                        to_min_sec_millis_str(duration),
                    ),
                    (_, _) => (0.0, 1.0, "NA".to_string(), "NA".to_string()),
                };

                let progress_bar = ui.add(
                    egui::ProgressBar::new((position / duration) as f32)
                        .text(format!("{} / {}", position_display, duration_display))
                        .rounding(Rounding::default()),
                );

                if let Some(click_position) = progress_bar
                    .interact(egui::Sense::click())
                    .interact_pointer_pos()
                {
                    let relative_x = click_position.x - progress_bar.interact_rect.left();
                    let relative_percent = relative_x / progress_bar.interact_rect.width();
                    controller.handle_event(app_data, BoothEvent::SeekOne(relative_percent as f64));
                }

                ui.horizontal(|ui| {
                    let mut ch_one = app_data.mixer.get_ch_one_volume();
                    ui.add(
                        egui::Slider::new(&mut ch_one, 0.0..=1.0)
                            .text("Ch ONE")
                            .vertical(),
                    );
                    controller.handle_event(app_data, BoothEvent::VolumeOneChanged(ch_one));

                    let mut pitch_one = app_data.turntable_one.pitch();
                    ui.add(
                        egui::Slider::new(&mut pitch_one, 1.08..=0.92)
                            .text("PITCH ONE")
                            .vertical(),
                    );
                    controller.handle_event(app_data, BoothEvent::PitchOneChanged(pitch_one));

                    let mut eq_low_one = app_data.mixer.get_eq_low_one_gain();
                    ui.add(
                        egui::Slider::new(&mut eq_low_one, -24.0..=3.0)
                            .text("LOW ONE")
                            .vertical(),
                    );
                    controller.handle_event(app_data, BoothEvent::EqLowOneChanged(eq_low_one));

                    let mut eq_high_one = app_data.mixer.get_eq_high_one_gain();
                    ui.add(
                        egui::Slider::new(&mut eq_high_one, -24.0..=3.0)
                            .text("HIGH ONE")
                            .vertical(),
                    );
                    controller.handle_event(app_data, BoothEvent::EqHighOneChanged(eq_high_one));

                    if app_data.cover_one.create_texture(ctx) {
                        log::info!("Cover one texture created");
                    }
                    match app_data.cover_one.texture() {
                        Some(texture) => ui.add(
                            Image::new((texture.id(), texture.size_vec2()))
                                .rounding(10.0)
                                .shrink_to_fit(),
                        ),
                        None => ui.add(Label::new("No Cover")),
                    };
                });

                let cue_one = app_data.mixer.is_cue_one_enabled();
                if ui
                    .add(egui::Button::new("Cue").fill(if cue_one {
                        egui::Color32::LIGHT_BLUE
                    } else {
                        theme_visuals.widgets.inactive.weak_bg_fill
                    }))
                    .clicked()
                {
                    controller.handle_event(app_data, BoothEvent::ToggleCueOne);
                }

                if ui
                    .add(
                        egui::Button::new("Focus ChOne").fill(match app_data.turntable_focus {
                            TurntableFocus::One => egui::Color32::from_rgb(170, 170, 255),
                            _ => theme_visuals.widgets.inactive.weak_bg_fill,
                        }),
                    )
                    .clicked()
                {
                    controller
                        .handle_event(app_data, BoothEvent::FocusChanged(TurntableFocus::One));
                }

                if ui.add(egui::Button::new("START-STOP")).clicked() {
                    controller.handle_event(app_data, BoothEvent::ToggleStartStopOne);
                }
            });

            cols[1].vertical_centered_justified(|ui| {
                ui.with_layout(Layout::top_down_justified(egui::Align::LEFT), |ui| {
                    ui.add(match app_data.turntable_two.currently_loaded() {
                        Some(path) => Label::new(path.split('/').last().unwrap()),
                        None => Label::new("No Track Loaded"),
                    })
                });

                let (position, duration, position_display, duration_display) = match (
                    app_data.turntable_two.position(),
                    app_data.turntable_two.duration(),
                ) {
                    (Some(position), Some(duration)) => (
                        position,
                        duration,
                        to_min_sec_millis_str(position),
                        to_min_sec_millis_str(duration),
                    ),
                    (_, _) => (0.0, 1.0, "NA".to_string(), "NA".to_string()),
                };

                let progress_bar = ui.add(
                    egui::ProgressBar::new((position / duration) as f32)
                        .text(format!("{} / {}", position_display, duration_display))
                        .rounding(Rounding::default()),
                );

                if let Some(click_position) = progress_bar
                    .interact(egui::Sense::click())
                    .interact_pointer_pos()
                {
                    let relative_x = click_position.x - progress_bar.interact_rect.left();
                    let relative_percent = relative_x / progress_bar.interact_rect.width();
                    controller.handle_event(app_data, BoothEvent::SeekTwo(relative_percent as f64));
                }

                ui.horizontal(|ui| {
                    let mut ch_two = app_data.mixer.get_ch_two_volume();
                    ui.add(
                        egui::Slider::new(&mut ch_two, 0.0..=1.0)
                            .text("Ch TWO")
                            .vertical(),
                    );
                    controller.handle_event(app_data, BoothEvent::VolumeTwoChanged(ch_two));

                    let mut pitch_two = app_data.turntable_two.pitch();
                    ui.add(
                        egui::Slider::new(&mut pitch_two, 1.08..=0.92)
                            .text("PITCH TWO")
                            .vertical(),
                    );
                    controller.handle_event(app_data, BoothEvent::PitchTwoChanged(pitch_two));

                    let mut eq_low_two = app_data.mixer.get_eq_low_two_gain();
                    ui.add(
                        egui::Slider::new(&mut eq_low_two, -24.0..=3.0)
                            .text("LOW TWO")
                            .vertical(),
                    );
                    controller.handle_event(app_data, BoothEvent::EqLowTwoChanged(eq_low_two));

                    let mut eq_high_two = app_data.mixer.get_eq_high_two_gain();
                    ui.add(
                        egui::Slider::new(&mut eq_high_two, -24.0..=3.0)
                            .text("HIGH TWO")
                            .vertical(),
                    );
                    controller.handle_event(app_data, BoothEvent::EqHighTwoChanged(eq_high_two));

                    if app_data.cover_two.create_texture(ctx) {
                        log::info!("Cover two texture created");
                    }
                    match app_data.cover_two.texture() {
                        Some(texture) => ui.add(
                            Image::new((texture.id(), texture.size_vec2()))
                                .rounding(10.0)
                                .shrink_to_fit(),
                        ),
                        None => ui.add(Label::new("No Cover")),
                    };
                });

                let cue_two = app_data.mixer.is_cue_two_enabled();
                if ui
                    .add(egui::Button::new("Cue").fill(if cue_two {
                        egui::Color32::LIGHT_BLUE
                    } else {
                        theme_visuals.widgets.inactive.weak_bg_fill
                    }))
                    .clicked()
                {
                    controller.handle_event(app_data, BoothEvent::ToggleCueTwo);
                }

                if ui
                    .add(
                        egui::Button::new("Focus ChTwo").fill(match app_data.turntable_focus {
                            TurntableFocus::Two => egui::Color32::from_rgb(170, 170, 255),
                            _ => theme_visuals.widgets.inactive.weak_bg_fill,
                        }),
                    )
                    .clicked()
                {
                    controller
                        .handle_event(app_data, BoothEvent::FocusChanged(TurntableFocus::Two));
                }

                if ui.add(egui::Button::new("START-STOP")).clicked() {
                    controller.handle_event(app_data, BoothEvent::ToggleStartStopTwo);
                }
            });
        });
    });

    if app_data.show_debug_panel {
        egui::TopBottomPanel::bottom("debug_panel").show(ctx, |ui| {
            ui.label("Debug Panel");
            ui.separator();
            ui.label(format!("frame_counter: {}", app_data.frame_counter));
            ui.label(format!("focus: {:?}", app_data.turntable_focus));
            ui.label(format!("window_size: {:?}", window.inner_size()));
            ui.label(format!("modifiers_key: {:?}", app_data.modifiers_key));
        });
    }
}
