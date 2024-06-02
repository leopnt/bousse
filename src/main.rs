use std::{
    error::Error,
    time::{Duration, Instant},
};

use processable::Processable;
use winit::{
    event::{Event, StartCause},
    event_loop::{ControlFlow, EventLoop},
};

mod app;
mod controller;
mod gpu;
mod gui;
mod mixer;
mod processable;
mod turntable;
mod utils;

use app::App;
use dotenv::dotenv;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    dotenv().ok();

    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::wait_duration(Duration::default()));

    let mut app = App::new(&event_loop);

    let mut start = Instant::now();
    event_loop.run(move |event, elwt| match event {
        Event::DeviceEvent { event, .. } => app.on_device_event(event),
        Event::WindowEvent { event, .. } => app.on_window_event(event, elwt),
        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
            app.process(start.elapsed().as_secs_f64());
            start = Instant::now();
            app.on_resume_time_reached(elwt)
        }
        _ => (),
    })?;

    Ok(())
}
