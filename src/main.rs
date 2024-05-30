use std::{error::Error, time::Duration};

use winit::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop},
};

mod utils;
mod app;
mod gpu;
mod gui;
mod mixer;

use app::App;
use dotenv::dotenv;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    dotenv().ok();

    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::wait_duration(Duration::default()));

    let mut app = App::new(&event_loop);

    event_loop.run(move |event, elwt| match event {
        Event::AboutToWait => {app.app_vars.mixer.process()},
        Event::DeviceEvent { event, .. } => app.on_device_event(event),
        Event::WindowEvent { event, .. } => app.on_window_event(event, elwt),
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => app.on_resume_time_reached(elwt),
        _ => (),
    })?;

    Ok(())
}
