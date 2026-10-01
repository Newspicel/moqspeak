//! Opening, listing and switching the microphone and speakers.

use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use super::{AudioShared, FRAME, Mixer, RATE, Resampler};

/// What is open, for the UI.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceInfo {
    pub input: Option<String>,
    pub output: Option<String>,
    pub error: Option<String>,
}

/// The devices the system offers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeviceLists {
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub default_input: Option<String>,
    pub default_output: Option<String>,
}

fn name_of(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|d| d.name().to_owned())
}

/// Lists microphones and speakers by name.
pub fn list_devices() -> DeviceLists {
    let host = cpal::default_host();
    let mut lists = DeviceLists {
        default_input: host.default_input_device().as_ref().and_then(name_of),
        default_output: host.default_output_device().as_ref().and_then(name_of),
        ..Default::default()
    };
    if let Ok(devices) = host.input_devices() {
        lists.inputs = devices.filter_map(|d| name_of(&d)).collect();
    }
    if let Ok(devices) = host.output_devices() {
        lists.outputs = devices.filter_map(|d| name_of(&d)).collect();
    }
    lists.inputs.dedup();
    lists.outputs.dedup();
    lists
}

fn find_device(input: bool, name: Option<&str>) -> Result<cpal::Device> {
    let host = cpal::default_host();
    if let Some(wanted) = name {
        let mut devices = if input { host.input_devices()? } else { host.output_devices()? };
        if let Some(d) = devices.find(|d| name_of(d).as_deref() == Some(wanted)) {
            return Ok(d);
        }
        tracing::warn!("audio device \"{wanted}\" not found; using the default");
    }
    let device = if input { host.default_input_device() } else { host.default_output_device() };
    device.ok_or_else(|| anyhow!(if input { "no microphone found" } else { "no speakers found" }))
}

fn pick_config(
    configs: impl Iterator<Item = cpal::SupportedStreamConfigRange>,
    fallback: cpal::SupportedStreamConfig,
) -> cpal::SupportedStreamConfig {
    let mut best: Option<cpal::SupportedStreamConfig> = None;
    for range in configs {
        if range.sample_format() != SampleFormat::F32 {
            continue;
        }
        if let Some(c) = range.try_with_sample_rate(RATE) {
            if best.as_ref().is_none_or(|b| c.channels() < b.channels()) {
                best = Some(c);
            }
        }
    }
    best.unwrap_or(fallback)
}

fn to_f32<T: cpal::SizedSample>(data: &[T]) -> Vec<f32>
where
    f32: cpal::FromSample<T>,
{
    data.iter().map(|s| <f32 as cpal::FromSample<T>>::from_sample_(*s)).collect()
}

fn build_input(
    name: Option<&str>,
    shared: Arc<AudioShared>,
    frames: Sender<Vec<f32>>,
) -> Result<(Stream, String)> {
    let device = find_device(true, name)?;
    let default = device.default_input_config()?;
    let preferred = pick_config(device.supported_input_configs()?, default.clone());
    match open_input(&device, preferred, shared.clone(), frames.clone()) {
        Ok(s) => Ok(s),
        Err(e) => {
            tracing::info!("microphone at 48 kHz failed ({e:#}); using its default rate");
            open_input(&device, default, shared, frames)
        }
    }
}

fn open_input(
    device: &cpal::Device,
    config: cpal::SupportedStreamConfig,
    shared: Arc<AudioShared>,
    frames: Sender<Vec<f32>>,
) -> Result<(Stream, String)> {
    let name = name_of(device).unwrap_or_else(|| "Microphone".into());
    let format = config.sample_format();
    let stream_config: StreamConfig = config.into();
    let channels = stream_config.channels as usize;
    let mut resampler = Resampler::new(stream_config.sample_rate, RATE);
    let mut scratch = Vec::new();

    let mut handle = move |interleaved: Vec<f32>| {
        let gain = shared.input_gain.get();
        let mono: Vec<f32> = interleaved
            .chunks(channels)
            .map(|c| c.iter().sum::<f32>() / channels as f32 * gain)
            .collect();
        scratch.clear();
        resampler.process(&mono, &mut scratch);
        let _ = frames.send(std::mem::take(&mut scratch));
    };
    let err = |e| tracing::warn!("input stream error: {e}");
    let stream = match format {
        SampleFormat::F32 => {
            device.build_input_stream(stream_config, move |d: &[f32], _: &_| handle(d.to_vec()), err, None)?
        }
        SampleFormat::I16 => {
            device.build_input_stream(stream_config, move |d: &[i16], _: &_| handle(to_f32(d)), err, None)?
        }
        SampleFormat::I32 => {
            device.build_input_stream(stream_config, move |d: &[i32], _: &_| handle(to_f32(d)), err, None)?
        }
        other => return Err(anyhow!("unsupported microphone sample format {other}")),
    };
    stream.play().context("starting microphone")?;
    Ok((stream, name))
}

fn build_output(name: Option<&str>, shared: Arc<AudioShared>, mixer: Arc<Mixer>) -> Result<(Stream, String)> {
    let device = find_device(false, name)?;
    let default = device.default_output_config()?;
    let preferred = pick_config(device.supported_output_configs()?, default.clone());
    match open_output(&device, preferred, shared.clone(), mixer.clone()) {
        Ok(s) => Ok(s),
        Err(e) => {
            tracing::info!("speakers at 48 kHz failed ({e:#}); using their default rate");
            open_output(&device, default, shared, mixer)
        }
    }
}

fn open_output(
    device: &cpal::Device,
    config: cpal::SupportedStreamConfig,
    shared: Arc<AudioShared>,
    mixer: Arc<Mixer>,
) -> Result<(Stream, String)> {
    let name = name_of(device).unwrap_or_else(|| "Speakers".into());
    if config.sample_format() != SampleFormat::F32 {
        return Err(anyhow!("unsupported speaker sample format {}", config.sample_format()));
    }
    let stream_config: StreamConfig = config.into();
    let channels = stream_config.channels as usize;
    let device_rate = stream_config.sample_rate;
    let mut resampler = Resampler::new(RATE, device_rate);
    let mut pending: VecDeque<f32> = VecDeque::new();
    let mut codec = vec![0f32; FRAME];
    let mut converted = Vec::new();

    let stream = device.build_output_stream(
        stream_config,
        move |data: &mut [f32], _: &_| {
            let frames = data.len() / channels;
            let volume = if shared.deafened.load(Ordering::Relaxed) { 0.0 } else { shared.master_volume.get() };
            while pending.len() < frames {
                // Mix in small codec-rate blocks so latency stays low.
                let want = ((frames - pending.len()) as f64 * RATE as f64 / device_rate as f64)
                    .ceil()
                    .clamp(16.0, FRAME as f64) as usize;
                codec.resize(want, 0.0);
                mixer.mix(&mut codec);
                converted.clear();
                resampler.process(&codec, &mut converted);
                pending.extend(converted.iter());
            }
            for frame in data.chunks_mut(channels) {
                let s = (pending.pop_front().unwrap_or(0.0) * volume).clamp(-1.0, 1.0);
                frame.fill(s);
            }
        },
        |e| tracing::warn!("output stream error: {e}"),
        None,
    )?;
    stream.play().context("starting speakers")?;
    Ok((stream, name))
}

enum Cmd {
    Input(Option<String>),
    Output(Option<String>),
}

/// The thread that owns the streams. cpal streams are not `Send` on every platform.
pub(super) struct DeviceThread {
    commands: Mutex<Sender<(Cmd, Sender<DeviceInfo>)>>,
    info: Arc<Mutex<DeviceInfo>>,
}

impl DeviceThread {
    pub(super) fn spawn(
        shared: Arc<AudioShared>,
        mixer: Arc<Mixer>,
        frames: Sender<Vec<f32>>,
        input: Option<String>,
        output: Option<String>,
    ) -> Self {
        let (tx, rx) = channel::<(Cmd, Sender<DeviceInfo>)>();
        let info = Arc::new(Mutex::new(DeviceInfo::default()));
        let (ready_tx, ready_rx) = channel();
        {
            let info = info.clone();
            std::thread::Builder::new()
                .name("audio-devices".into())
                .spawn(move || run(shared, mixer, frames, input, output, info, rx, ready_tx))
                .expect("spawn audio device thread");
        }
        if ready_rx.recv_timeout(Duration::from_secs(5)).is_err() {
            info.lock().unwrap().error = Some("audio devices did not open in time".into());
        }
        Self { commands: Mutex::new(tx), info }
    }

    pub(super) fn info(&self) -> DeviceInfo {
        self.info.lock().unwrap().clone()
    }

    fn ask(&self, cmd: Cmd) -> DeviceInfo {
        let (reply_tx, reply_rx) = channel();
        let _ = self.commands.lock().unwrap().send((cmd, reply_tx));
        reply_rx.recv_timeout(Duration::from_secs(5)).unwrap_or_else(|_| self.info())
    }

    pub(super) fn set_input(&self, name: Option<String>) -> DeviceInfo {
        self.ask(Cmd::Input(name))
    }

    pub(super) fn set_output(&self, name: Option<String>) -> DeviceInfo {
        self.ask(Cmd::Output(name))
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    shared: Arc<AudioShared>,
    mixer: Arc<Mixer>,
    frames: Sender<Vec<f32>>,
    input: Option<String>,
    output: Option<String>,
    info: Arc<Mutex<DeviceInfo>>,
    commands: Receiver<(Cmd, Sender<DeviceInfo>)>,
    ready: Sender<()>,
) {
    let mut input_error = None;
    let mut output_error = None;
    let open_in = |name: Option<&str>, err: &mut Option<String>| match build_input(name, shared.clone(), frames.clone()) {
        Ok(s) => {
            *err = None;
            Some(s)
        }
        Err(e) => {
            *err = Some(format!("microphone: {e:#}"));
            None
        }
    };
    let open_out = |name: Option<&str>, err: &mut Option<String>| match build_output(name, shared.clone(), mixer.clone()) {
        Ok(s) => {
            *err = None;
            Some(s)
        }
        Err(e) => {
            *err = Some(format!("speakers: {e:#}"));
            None
        }
    };
    let mut mic = open_in(input.as_deref(), &mut input_error);
    let mut speakers = open_out(output.as_deref(), &mut output_error);

    let publish = |mic: &Option<(Stream, String)>, speakers: &Option<(Stream, String)>, ie: &Option<String>, oe: &Option<String>| {
        let errors: Vec<String> = [ie.clone(), oe.clone()].into_iter().flatten().collect();
        let next = DeviceInfo {
            input: mic.as_ref().map(|(_, n)| n.clone()),
            output: speakers.as_ref().map(|(_, n)| n.clone()),
            error: (!errors.is_empty()).then(|| errors.join("; ")),
        };
        if let Some(e) = &next.error {
            tracing::warn!("{e}");
        }
        *info.lock().unwrap() = next.clone();
        next
    };
    publish(&mic, &speakers, &input_error, &output_error);
    let _ = ready.send(());

    while let Ok((cmd, reply)) = commands.recv() {
        match cmd {
            Cmd::Input(name) => {
                drop(mic.take());
                mic = open_in(name.as_deref(), &mut input_error);
            }
            Cmd::Output(name) => {
                drop(speakers.take());
                speakers = open_out(name.as_deref(), &mut output_error);
            }
        }
        let _ = reply.send(publish(&mic, &speakers, &input_error, &output_error));
    }
}
