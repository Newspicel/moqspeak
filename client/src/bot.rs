//! A headless participant for testing: `moqspeak --bot <address> <nickname> [channel]`.
//!
//! It joins the server, moves to the named channel, beeps a short tone every two seconds over
//! MoQ, and prints who it hears.

use std::f32::consts::TAU;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::audio::{Audio, FRAME, Packet, RATE};
use crate::engine::{Command, Engine, Event};
use crate::model::ClientMsg;

pub fn run(args: &[String]) {
    let address = args
        .first()
        .cloned()
        .unwrap_or_else(|| crate::ui::state::DEFAULT_ADDRESS.into());
    let nickname = args.get(1).cloned().unwrap_or_else(|| "BeepBot".into());
    let channel = args.get(2).cloned();

    let (packet_tx, packet_rx) = tokio::sync::mpsc::unbounded_channel();
    let audio = Arc::new(Audio::headless());
    let shared = audio.shared.clone();

    // The tone: 600 ms of 440/660 Hz every two seconds.
    std::thread::spawn(move || {
        let mut encoder =
            opus::Encoder::new(RATE, opus::Channels::Mono, opus::Application::Voip).expect("opus");
        let start = Instant::now();
        let mut seq = 0u32;
        let mut phase = 0f32;
        let mut buf = [0u8; 1500];
        let mut next = Instant::now();
        loop {
            next += Duration::from_millis(20);
            let t = start.elapsed().as_millis() % 2000;
            let on = t < 600;
            shared.transmitting.store(on, Ordering::Relaxed);
            if on {
                let freq = if t < 300 { 440.0 } else { 660.0 };
                let frame: Vec<f32> = (0..FRAME)
                    .map(|_| {
                        phase = (phase + TAU * freq / RATE as f32) % TAU;
                        phase.sin() * 0.25
                    })
                    .collect();
                if let Ok(n) = encoder.encode_float(&frame, &mut buf) {
                    let _ = packet_tx.send(Packet {
                        seq,
                        data: buf[..n].to_vec(),
                    });
                    seq = seq.wrapping_add(1);
                }
            } else {
                seq = seq.wrapping_add(1);
            }
            std::thread::sleep(next.saturating_duration_since(Instant::now()));
        }
    });

    let (engine, mut events) = Engine::start_with(audio, packet_rx);
    engine.send(Command::Connect { address, nickname });
    let mut joined = false;
    let mut names = std::collections::HashMap::new();
    while let Some(event) = events.blocking_recv() {
        match event {
            Event::State {
                channels, clients, ..
            } => {
                for c in &clients {
                    names.insert(c.id, c.name.clone());
                }
                if !joined {
                    if let Some(wanted) = &channel {
                        if let Some(ch) = channels
                            .iter()
                            .find(|c| c.name.eq_ignore_ascii_case(wanted))
                        {
                            engine.send(Command::Send(ClientMsg::Join { channel: ch.id }));
                        }
                    }
                    joined = true;
                }
                println!("[state] {} clients online", clients.len());
            }
            Event::Talking(set) => {
                let who: Vec<String> = set
                    .iter()
                    .map(|id| names.get(id).cloned().unwrap_or_else(|| id.to_string()))
                    .collect();
                println!("[talking] {}", who.join(", "));
            }
            Event::Chat {
                from_name, text, ..
            } => println!("[chat] {from_name}: {text}"),
            other => println!("[event] {other:?}"),
        }
    }
}
