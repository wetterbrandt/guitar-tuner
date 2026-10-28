use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct SharedSamples {
    inner: Arc<Mutex<VecDeque<f32>>>,
    capacity: usize,
}

impl SharedSamples {
    fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    fn push_interleaved(&self, data: &[f32], channels: usize) {
        let Ok(mut buffer) = self.inner.try_lock() else {
            return;
        };
        for frame in data.chunks(channels) {
            buffer.push_back(frame.iter().sum::<f32>() / channels as f32);
        }
        while buffer.len() > self.capacity {
            buffer.pop_front();
        }
    }

    pub fn snapshot(&self, wanted: usize) -> Vec<f32> {
        let Ok(buffer) = self.inner.lock() else {
            return Vec::new();
        };
        buffer
            .iter()
            .skip(buffer.len().saturating_sub(wanted))
            .copied()
            .collect()
    }
}

pub struct AudioInput {
    pub devices: Vec<String>,
    pub selected: usize,
    pub sample_rate: u32,
    pub samples: SharedSamples,
    pub error: Option<String>,
    _stream: Option<Stream>,
}

impl AudioInput {
    pub fn new() -> Self {
        let devices = input_devices().unwrap_or_default();
        let selected = devices
            .iter()
            .position(|name| {
                let name = name.to_lowercase();
                name.contains("blackstar") || name.contains("polar")
            })
            .unwrap_or(0);
        let mut input = Self {
            devices,
            selected,
            sample_rate: 48_000,
            samples: SharedSamples::new(48_000),
            error: None,
            _stream: None,
        };
        input.start(selected);
        input
    }

    pub fn start(&mut self, index: usize) {
        self._stream = None;
        self.selected = index;
        match open_device(index) {
            Ok((stream, sample_rate, samples)) => {
                self.sample_rate = sample_rate;
                self.samples = samples;
                self.error = None;
                self._stream = Some(stream);
            }
            Err(error) => self.error = Some(error.to_string()),
        }
    }
}

fn input_devices() -> Result<Vec<String>> {
    Ok(cpal::default_host()
        .input_devices()?
        .filter_map(|d| d.name().ok())
        .collect())
}

fn open_device(index: usize) -> Result<(Stream, u32, SharedSamples)> {
    let host = cpal::default_host();
    let device = host
        .input_devices()?
        .nth(index)
        .or_else(|| host.default_input_device())
        .ok_or_else(|| anyhow!("No audio input device was found"))?;
    let supported = device
        .default_input_config()
        .context("Could not read the input format")?;
    let config: StreamConfig = supported.clone().into();
    let sample_rate = config.sample_rate.0;
    let channels = config.channels as usize;
    let samples = SharedSamples::new(sample_rate as usize);
    let on_error = |error| eprintln!("Audio input error: {error}");

    let stream = match supported.sample_format() {
        SampleFormat::F32 => {
            let sink = samples.clone();
            device.build_input_stream(
                &config,
                move |data: &[f32], _| sink.push_interleaved(data, channels),
                on_error,
                None,
            )?
        }
        SampleFormat::I16 => {
            let sink = samples.clone();
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let converted: Vec<f32> =
                        data.iter().map(|&x| x as f32 / i16::MAX as f32).collect();
                    sink.push_interleaved(&converted, channels);
                },
                on_error,
                None,
            )?
        }
        SampleFormat::U16 => {
            let sink = samples.clone();
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    let converted: Vec<f32> =
                        data.iter().map(|&x| x as f32 / 32768.0 - 1.0).collect();
                    sink.push_interleaved(&converted, channels);
                },
                on_error,
                None,
            )?
        }
        format => return Err(anyhow!("Unsupported input sample format: {format:?}")),
    };
    stream.play().context("Could not start the audio input")?;
    Ok((stream, sample_rate, samples))
}
