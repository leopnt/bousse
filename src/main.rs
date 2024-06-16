use std::{error::Error, sync::Arc, time::Duration};

use egui::mutex::Mutex;
use midi_controller::MidiController;
use winit::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop},
};

mod app;
mod controller;
mod gpu;
mod gui;
mod midi_controller;
mod mixer;
mod processable;
mod turntable;
mod file_navigator;
mod utils;

use app::App;
use dotenv::dotenv;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    dotenv().ok();

    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::wait_duration(Duration::default()));

    let app = Arc::new(Mutex::new(App::new(&event_loop)));
    let app_clone = Arc::clone(&app);

    // the midi controller has to be kept alive during the whole execution of
    // the application, hence the named variable
    let _midi_controller = MidiController::new(
        move |message, app_clone| {
            app_clone.lock().on_midi_event(message);
        },
        app_clone,
    );

    event_loop.run(move |event, elwt| match event {
        Event::DeviceEvent { event, .. } => app.lock().on_device_event(event),
        Event::WindowEvent { event, .. } => app.lock().on_window_event(event, elwt),
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            app.lock().on_resume_time_reached(elwt)
        }
        _ => (),
    })?;

    Ok(())
}
