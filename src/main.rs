use std::{
    collections::HashMap,
    io::Write,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use anyhow::{Result, anyhow};
use midir::{MidiInput, MidiInputConnection};
use midly::{MidiMessage, live::LiveEvent};
use serialport5::SerialPort;
use tracing::{debug, error, info};

#[cfg(unix)]
use midir::os::unix::VirtualInput;

fn send_serial(message: String, serial_port: &mut Option<SerialPort>) {
    if let Some(serial_port) = serial_port {
        writeln!(serial_port, "{message}").expect("failed to send serial message");
    }
}

fn handle_event(timestamp: u64, data: &[u8], serial_port: &mut Option<SerialPort>) {
    debug!("received data at {}", timestamp);

    let event = LiveEvent::parse(data).unwrap();

    debug!("midi event {:?}", event);

    if let LiveEvent::Midi { channel, message } = event {
        match message {
            MidiMessage::NoteOn { key, .. } => {
                info!("note on {} on channel {}", key, channel);
                send_serial(format!("on {key}"), serial_port);
            }
            MidiMessage::NoteOff { key, .. } => {
                info!("note off {} on channel {}", key, channel);
                send_serial(format!("off {key}"), serial_port);
            }
            _ => (),
        }
    }
}

fn use_loopback(
    serial_port: Option<SerialPort>,
) -> Result<MidiInputConnection<Option<SerialPort>>> {
    let mut rl = rustyline::DefaultEditor::new()?;

    let midi_input = MidiInput::new("drummond")?;

    let mut port_map = HashMap::new();

    for port in midi_input.ports() {
        let port_id = port.id();
        let port_name = midi_input.port_name(&port)?;

        info!("port '{}' with id '{}'", port_name, port_id);
        port_map.insert(port_name, port);
    }

    let port_name = rl.readline("name of port to connect to? > ")?;

    if !port_map.contains_key(&port_name) {
        error!("unknown port '{}'", port_name);
        return Err(anyhow!("unknown port"));
    }

    let port = port_map.get(&port_name).unwrap();

    let midi_connection = midi_input
        .connect(port, "drummond_connection", handle_event, serial_port)
        .map_err(|_| anyhow!("failed to connect to {}", port_name))?;

    info!("opened midi connection");

    Ok(midi_connection)
}

#[cfg(unix)]
fn use_virtual(serial_port: Option<SerialPort>) -> Result<MidiInputConnection<Option<SerialPort>>> {
    let midi_input = MidiInput::new("drummond")?;

    let midi_connection = midi_input
        .create_virtual("drummond", handle_event, serial_port)
        .map_err(|_| anyhow!("failed to open virtual input"))?;

    info!("opened midi connection");

    Ok(midi_connection)
}

#[cfg(windows)]
fn use_virtual(
    _serial_port: Option<SerialPort>,
) -> Result<MidiInputConnection<Option<SerialPort>>> {
    error!("virtual midi io is unavaliable on windows!");
    Err(anyhow!("poorly made os"))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    info!("hello world");

    let mut rl = rustyline::DefaultEditor::new()?;

    for port in serialport5::available_ports()? {
        info!("serial port available at: '{}'", port.port_name);
    }

    let serial_port_id = rl.readline("what serial port to use? ('none' for debugging) > ")?;

    let serial_port = if &serial_port_id == "none" {
        None
    } else {
        Some(SerialPort::builder().baud_rate(9600).open(&serial_port_id)?)
    };

    let connection_type = rl.readline("use 'loopback' (windows) or 'virtual' (unix)? > ")?;

    let midi_connection = match connection_type.as_str() {
        "loopback" => use_loopback(serial_port)?,
        "virtual" => use_virtual(serial_port)?,
        other => {
            error!("invalid connection type '{}'", other);
            return Err(anyhow!("invalid user input"));
        }
    };

    drop(rl);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::try_set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }

    midi_connection.close();
    info!("closed midi connection");

    Ok(())
}
