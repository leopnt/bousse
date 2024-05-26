use std::{error::Error, time::Duration};

use winit::{
    event::{Event, KeyEvent, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

mod app;
mod gpu;
mod gui;
use app::App;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::wait_duration(Duration::default()));

    let mut app = App::new(&event_loop);

    event_loop.run(move |event, elwt| match event {
        Event::AboutToWait => {}

        Event::WindowEvent {
            event: WindowEvent::ModifiersChanged(modifiers),
            ..
        } => {
            app.on_modifiers_key_changed(modifiers);
        }

        Event::WindowEvent {
            event:
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key,
                            state,
                            repeat,
                            ..
                        },
                    ..
                },
            ..
        } => {
            app.on_physical_key_event(physical_key, state, repeat);
        }

        Event::WindowEvent { event, .. } => {
            app.on_window_event(event, elwt);
        }

        Event::NewEvents(StartCause::ResumeTimeReached { .. }) => app.on_resume_time_reached(elwt),

        _ => (),
    })?;

    Ok(())
}
