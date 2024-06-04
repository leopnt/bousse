use std::{io::stdin, sync::Arc};

use egui::mutex::Mutex;
use midir::{Ignore, MidiInput, MidiInputConnection};

use crate::app::App;

pub struct MidiController {
    _conn_in: MidiInputConnection<Arc<Mutex<App>>>,
}

impl MidiController {
    pub fn new<F>(f: F, app_clone: Arc<Mutex<App>>) -> Self
    where
        F: Fn(&[u8], &Arc<Mutex<App>>) + Send + 'static,
    {
        let mut midi_in = MidiInput::new("midir reading input").unwrap();
        midi_in.ignore(Ignore::None);

        let in_ports = midi_in.ports();
        let in_port = match in_ports.len() {
            0 => panic!("No MIDI Input port found"),
            1 => {
                println!(
                    "Choosing the only available input port: {}",
                    midi_in.port_name(&in_ports[0]).unwrap()
                );
                &in_ports[0]
            }
            _ => {
                println!("\nAvailable MIDI input ports:");
                for (i, p) in in_ports.iter().enumerate() {
                    println!("{}: {}", i, midi_in.port_name(p).unwrap());
                }
                print!("Please select MIDI input port: ");
                let mut input = String::new();
                stdin().read_line(&mut input).unwrap();
                in_ports
                    .get(input.trim().parse::<usize>().unwrap())
                    .ok_or("invalid MIDI input port selected")
                    .unwrap()
            }
        };

        println!("\nOpening MIDI connection");
        let in_port_name = midi_in.port_name(in_port).unwrap();

        let _conn_in = midi_in
            .connect(
                in_port,
                "midir-read-input",
                move |_, message, app| {
                    f(message, app);
                },
                app_clone,
            )
            .unwrap();

        println!(
            "Connection open, reading MIDI input from '{}'",
            in_port_name
        );

        Self { _conn_in }
    }
}
