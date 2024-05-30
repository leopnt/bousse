use std::env;
use std::sync::Arc;
use std::time::Duration;

use egui_wgpu::ScreenDescriptor;
use winit::event::{DeviceEvent, ElementState, KeyEvent, Modifiers, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Window, WindowBuilder};

use kira::manager::{backend::DefaultBackend, AudioManager, AudioManagerSettings};
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle, StreamingSoundSettings};
use kira::sound::FromFileError;
use kira::track::TrackHandle;
use kira::track::{TrackBuilder, TrackRoutes};
use kira::tween::Tween;

use crate::gpu::Gpu;
use crate::gui::Gui;

struct AppVariables {
    pub fps: u8,
    pub frame_counter: u32,
    pub show_debug_panel: bool,
    pub modifiers_key: Modifiers,
    pub audio_manager: AudioManager,
    pub master: TrackHandle,
    pub cue: TrackHandle,
    pub cue_mix: f64,
    pub sound_one: StreamingSoundHandle<FromFileError>,
    pub track_one: TrackHandle,
    pub cue_one: bool,
    pub ch_one: f64,
}

pub struct App {
    pub window: Arc<Window>,
    pub gpu: Gpu,
    pub gui: Gui,
    app_vars: AppVariables,
}

/// Explode a given value between 0 and 1 into respective mixed values
fn cue_crossfade(norm_value: f64) -> (f64, f64) {
    (1. - norm_value, norm_value)
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

        let mut manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();

        let master = manager.add_sub_track(TrackBuilder::new()).unwrap();
        let cue = manager.add_sub_track(TrackBuilder::new()).unwrap();

        let track_one = manager
            .add_sub_track(
                TrackBuilder::new().volume(1.).routes(
                    TrackRoutes::empty()
                        .with_route(&master, 0.0)
                        .with_route(&cue, 0.0),
                ),
            )
            .unwrap();

        let settings = StreamingSoundSettings::new().output_destination(&track_one);

        let sound_path = env::var("SOUND_PATH").expect("SOUND_PATH environment variable not set");
        let sound = StreamingSoundData::from_file(sound_path).unwrap();

        let mut sound_one = manager.play(sound.with_settings(settings)).unwrap();

        let app_vars = AppVariables {
            fps: 24,
            frame_counter: 0,
            show_debug_panel: false,
            modifiers_key: Modifiers::default(),
            audio_manager: manager,
            sound_one: sound_one,
            master: master,
            cue: cue,
            track_one: track_one,
            cue_mix: 0.5,
            cue_one: false,
            ch_one: 0.0,
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
        println!("DEVICE EVENT: {:?}", event);
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
        ui.toggle_value(&mut app_vars.cue_one, "Cue ONE");

        ui.add(egui::Slider::new(&mut app_vars.cue_mix, 0.0..=1.0).text("Cue Mix"));
        ui.add(egui::Slider::new(&mut app_vars.ch_one, 0.0..=1.0).text("Ch ONE"));

        let (cue_volume, master_volume) = cue_crossfade(app_vars.cue_mix);

        app_vars.master.set_volume(master_volume, Tween::default());
        app_vars.cue.set_volume(cue_volume, Tween::default());

        app_vars
            .track_one
            .set_route(&app_vars.master, app_vars.ch_one, Tween::default())
            .unwrap();
        app_vars
            .track_one
            .set_route(
                &app_vars.cue,
                if app_vars.cue_one { 1.0 } else { 0.0 },
                Tween::default(),
            )
            .unwrap();
    });

    if app_vars.show_debug_panel {
        egui::TopBottomPanel::bottom("debug_panel").show(ctx, |ui| {
            ui.label("Debug Panel");
            ui.separator();
            ui.label(format!("frame_counter: {}", app_vars.frame_counter));
            ui.label(format!("window_size: {:?}", window.inner_size()));
            ui.label(format!(
                "audio_manager.num_sub_tracks: {:?}",
                app_vars.audio_manager.num_sub_tracks()
            ));
        });
    }
}
