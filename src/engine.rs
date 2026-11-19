use crate::{
    Destination, Source,
    capture::{Capture, Reader},
    output::{self, Output},
};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const RATE: usize = 48_000;
const FRAMES: usize = 480;
const PERIOD: Duration = Duration::from_millis(10);
type Devices = Vec<(String, String, usize)>;

#[derive(Default)]
struct Session {
    streams: Vec<(String, Output)>,
    recorder: Option<Output>,
    meters: [ChannelMeter; 2],
    present: bool,
}

#[derive(Default, Clone, Copy)]
pub struct MeterLevel {
    pub signal: f32,
    pub held: f32,
    pub clipped: bool,
}

#[derive(Default)]
struct ChannelMeter {
    level: MeterLevel,
    hold_remaining: Duration,
}

impl ChannelMeter {
    fn update(&mut self, peak: f32) {
        self.level.signal = peak.max(self.level.signal * 0.93);
        self.level.clipped |= peak >= 1.0;
        if peak >= self.level.held {
            self.level.held = peak;
            self.hold_remaining = Duration::from_millis(1500);
        } else if self.hold_remaining.is_zero() {
            // About 18 dB/s on the mixer's 10 ms clock.
            self.level.held = (self.level.held * 0.98).max(self.level.signal);
        } else {
            self.hold_remaining = self.hold_remaining.saturating_sub(PERIOD);
        }
    }
}

#[derive(Default)]
struct Mixer {
    sources: Vec<Source>,
    sessions: HashMap<u64, Session>,
    finalizing: Vec<(String, Output)>,
}

#[derive(Default)]
struct Inputs {
    devices: Devices,
    readers: HashMap<String, Reader>,
}

pub struct Engine {
    mixer: Arc<Mutex<Mixer>>,
    inputs: Arc<Mutex<Inputs>>,
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl Engine {
    pub fn new() -> Self {
        let mixer = Arc::new(Mutex::new(Mixer::default()));
        let inputs = Arc::new(Mutex::new(Inputs::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let manager = {
            let mixer = Arc::clone(&mixer);
            let inputs = Arc::clone(&inputs);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                let mut capture = Capture::new();
                while !stop.load(Ordering::Acquire) {
                    let wanted: HashSet<_> = mixer
                        .lock()
                        .unwrap()
                        .sources
                        .iter()
                        .map(|s| s.device.clone())
                        .filter(|id| !id.is_empty())
                        .collect();
                    capture.tick(&wanted.into_iter().collect::<Vec<_>>());
                    let snapshot = Inputs {
                        devices: capture.devices(),
                        readers: capture.readers(),
                    };
                    *inputs.lock().unwrap() = snapshot;
                    thread::sleep(Duration::from_millis(150));
                }
            })
        };
        let clock = {
            let mixer = Arc::clone(&mixer);
            let inputs = Arc::clone(&inputs);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                let mut converters = HashMap::<String, Converter>::new();
                let mut next = Instant::now();
                while !stop.load(Ordering::Acquire) {
                    let blocks = read_inputs(&inputs.lock().unwrap().readers, &mut converters);
                    mixer.lock().unwrap().render(&blocks);
                    next += PERIOD;
                    let now = Instant::now();
                    if now.saturating_duration_since(next) > Duration::from_millis(250) {
                        // Do not enqueue hours of obsolete audio after system sleep.
                        next = now;
                    }
                    thread::sleep(next.saturating_duration_since(now));
                }
            })
        };
        Self {
            mixer,
            inputs,
            stop,
            workers: vec![clock, manager],
        }
    }

    pub fn devices(&self) -> Devices {
        self.inputs.lock().unwrap().devices.clone()
    }
    pub fn update(&self, sources: &[Source]) {
        let mut mixer = self.mixer.lock().unwrap();
        let Mixer {
            sessions,
            sources: previous,
            ..
        } = &mut *mixer;
        sessions.retain(|id, session| {
            let Some(source) = sources.iter().find(|source| source.id == *id) else {
                return false;
            };
            if let Some(old) = previous.iter().find(|old| old.id == *id) {
                if old.device != source.device || old.left != source.left {
                    session.meters[0] = ChannelMeter::default();
                }
                if old.device != source.device || old.right != source.right {
                    session.meters[1] = ChannelMeter::default();
                }
            }
            true
        });
        mixer.sources = sources.to_vec();
    }
    pub fn streaming(&self, id: u64) -> bool {
        self.mixer
            .lock()
            .unwrap()
            .sessions
            .get(&id)
            .is_some_and(|s| s.streams.iter().any(|(_, output)| !output.finished()))
    }
    pub fn recording(&self, id: u64) -> bool {
        self.mixer
            .lock()
            .unwrap()
            .sessions
            .get(&id)
            .is_some_and(|s| s.recorder.as_ref().is_some_and(|rec| !rec.finished()))
    }
    pub fn present(&self, id: u64) -> bool {
        self.mixer
            .lock()
            .unwrap()
            .sessions
            .get(&id)
            .is_some_and(|s| s.present)
    }
    pub fn levels(&self, id: u64) -> [MeterLevel; 2] {
        self.mixer
            .lock()
            .unwrap()
            .sessions
            .get(&id)
            .map(|s| [s.meters[0].level, s.meters[1].level])
            .unwrap_or_default()
    }
    pub fn reset_clip(&self, id: u64, channel: usize) {
        if let Some(session) = self.mixer.lock().unwrap().sessions.get_mut(&id) {
            session.meters[channel].level.clipped = false;
        }
    }
    pub fn live_status(&self, id: u64) -> LiveStatus {
        let mixer = self.mixer.lock().unwrap();
        let mut status = LiveStatus::default();
        if let Some(session) = mixer.sessions.get(&id) {
            status.total = session.streams.len();
            for (index, (name, output)) in session.streams.iter().enumerate() {
                let sending = !output.finished() && !output.encoded_time().is_zero();
                let detail = output.status();
                status.connected += usize::from(sending);
                let name = if name.is_empty() {
                    format!("Sändare {}", index + 1)
                } else {
                    name.clone()
                };
                if !sending || detail != "Sänder" {
                    status.issues.push(format!("{name} · {detail}"));
                }
                status.targets.push(name);
            }
            if let Some(recorder) = &session.recorder {
                status.elapsed = recorder.encoded_time();
                status.record_detail = recorder.status();
            }
        }
        status
    }

    pub fn completed_recordings(&self) -> Vec<(String, Result<std::path::PathBuf, String>)> {
        let mut mixer = self.mixer.lock().unwrap();
        let mut results = Vec::new();
        mixer.finalizing.retain(|(name, output)| {
            if output.finished() {
                results.push((name.clone(), output.recording_result()));
                false
            } else {
                true
            }
        });
        results
    }
    pub fn set_stream(&self, source: &Source, destinations: &[Destination], start: bool) {
        let mut m = self.mixer.lock().unwrap();
        let s = m.sessions.entry(source.id).or_default();
        if !start {
            s.streams.clear();
        } else if s.streams.iter().all(|(_, output)| output.finished()) {
            s.streams = source
                .resolve_destinations(destinations)
                .into_iter()
                .map(|d| {
                    let label = d.name.trim().to_owned();
                    (label, Output::stream(d, channel_count(source)))
                })
                .collect();
        }
    }
    pub fn set_record(&self, source: &Source, start: bool) {
        let mut m = self.mixer.lock().unwrap();
        let s = m.sessions.entry(source.id).or_default();
        if !start {
            if let Some(mut recorder) = s.recorder.take() {
                recorder.stop();
                m.finalizing.push((source.name.clone(), recorder));
            }
        } else if s.recorder.as_ref().is_none_or(Output::finished) {
            s.recorder = Some(Output::record(
                &source.recording,
                &source.name,
                channel_count(source),
            ));
        }
    }
}

#[derive(Default)]
pub struct LiveStatus {
    pub connected: usize,
    pub total: usize,
    pub issues: Vec<String>,
    pub targets: Vec<String>,
    pub elapsed: Duration,
    pub record_detail: String,
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
        self.mixer.lock().unwrap().sessions.clear();
        output::wait_for_shutdown();
    }
}

struct Block {
    channels: usize,
    samples: Vec<f32>,
}

fn read_inputs(readers: &HashMap<String, Reader>, converters: &mut HashMap<String, Converter>) -> HashMap<String, Block> {
    converters.retain(|id, _| readers.contains_key(id));
    readers.iter().filter_map(|(id, reader)| {
        converters.entry(id.clone()).or_default().pull(reader).map(|block| (id.clone(), block))
    }).collect()
}

fn channel_count(source: &Source) -> u16 {
    if source.right.is_some() { 2 } else { 1 }
}

impl Mixer {
    fn render(&mut self, inputs: &HashMap<String, Block>) {
        for source in &self.sources {
            let session = self.sessions.entry(source.id).or_default();
            let input = inputs.get(&source.device);
            let (pcm, peak, present) = route(source, input);
            session.present = present;
            session.meters[0].update(peak.0);
            session.meters[1].update(peak.1);
            for output in session
                .streams
                .iter()
                .map(|(_, output)| output)
                .chain(session.recorder.iter())
            {
                output.push(&pcm);
            }
        }
    }
}

fn route(source: &Source, input: Option<&Block>) -> (Vec<f32>, (f32, f32), bool) {
    let channels = channel_count(source) as usize;
    let mut pcm = vec![0.0; FRAMES * channels];
    let mut peak = (0f32, 0f32);
    let Some(input) =
        input.filter(|b| source.left < b.channels && source.right.is_none_or(|r| r < b.channels))
    else {
        return (pcm, peak, false);
    };
    for (frame, out) in input
        .samples
        .chunks_exact(input.channels)
        .zip(pcm.chunks_exact_mut(channels))
    {
        out[0] = frame[source.left];
        peak.0 = peak.0.max(out[0].abs());
        if let Some(right) = source.right {
            out[1] = frame[right];
            peak.1 = peak.1.max(out[1].abs());
        }
    }
    (pcm, peak, true)
}

// Keep capture-device clocks independent of the fixed output timeline. A small
// prebuffer absorbs hardware callback jitter. Fractional phase survives ticks.
#[derive(Default)]
struct Converter {
    samples: Vec<f32>,
    phase: f64,
    channels: usize,
    rate: u32,
    ready: bool,
}

impl Converter {
    fn pull(&mut self, reader: &Reader) -> Option<Block> {
        let rate = reader.rate();
        let channels = reader.channels();
        if (rate, channels) != (self.rate, self.channels) {
            *self = Self {
                rate,
                channels,
                ..Default::default()
            };
        }
        let queued = reader.queued_frames()?;
        if !self.ready {
            if queued < rate as usize * 30 / 1000 {
                return None;
            }
            self.ready = true;
        }
        let step = rate as f64 / RATE as f64;
        // Gently recover excess latency after scheduling delays.
        let step = if queued > rate as usize / 12 {
            step * 1.002
        } else if queued < rate as usize / 60 {
            step * 0.999
        } else {
            step
        };
        let needed = (self.phase + FRAMES as f64 * step).ceil() as usize + 1;
        let count = needed.saturating_sub(self.samples.len() / channels);
        let (new, _, _) = reader.read(count)?;
        self.samples.extend(new);
        let available = self.samples.len() / channels;
        let mut out = vec![0.; FRAMES * channels];
        for frame in out.chunks_exact_mut(channels) {
            let i = self.phase as usize;
            if i >= available {
                self.ready = false;
                break;
            }
            let next = (i + 1).min(available - 1);
            let frac = (self.phase - i as f64) as f32;
            for (c, v) in frame.iter_mut().enumerate() {
                let a = self.samples[i * channels + c];
                let b = self.samples[next * channels + c];
                *v = a + (b - a) * frac;
            }
            self.phase += step;
        }
        let consumed = (self.phase as usize).min(available);
        self.samples.drain(..consumed * channels);
        self.phase -= consumed as f64;
        Some(Block {
            channels,
            samples: out,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_holds_short_peak_and_latches_each_channel_until_reset() {
        let mut mixer = Mixer::default();
        let source = Source {
            device: "mic".into(),
            right: Some(1),
            ..Source::default()
        };
        let id = source.id;
        mixer.sources.push(source);
        let mut samples = vec![0.0; FRAMES * 2];
        samples[0] = -1.0;
        samples[1] = 0.5;
        mixer.render(&HashMap::from([(
            "mic".into(),
            Block {
                channels: 2,
                samples,
            },
        )]));
        for _ in 0..150 {
            mixer.render(&HashMap::new());
        }
        let levels = &mut mixer.sessions.get_mut(&id).unwrap().meters;
        assert_eq!(levels[0].level.held, 1.0);
        assert!(levels[0].level.signal < 0.01);
        assert!(levels[0].level.clipped);
        assert!(!levels[1].level.clipped);
        levels[0].update(0.0);
        assert!(levels[0].level.held < 1.0);
        assert!(levels[0].level.clipped);
        levels[0].level.clipped = false;
        levels[0].update(0.1);
        assert!(!levels[0].level.clipped);
        levels[0].update(1.01);
        assert!(levels[0].level.clipped);
    }

    #[test]
    fn late_device_unplug_and_replug_preserve_timeline_and_channel_order() {
        let source = Source {
            left: 16,
            right: Some(2),
            ..Source::default()
        };
        let mut frame = [0.; 32];
        frame[16] = 0.75;
        frame[2] = -0.5;
        let input = Block {
            channels: 32,
            samples: frame.repeat(FRAMES),
        };
        for (block, expected) in [
            (None, [0., 0.]),
            (Some(&input), [0.75, -0.5]),
            (None, [0., 0.]),
            (Some(&input), [0.75, -0.5]),
        ] {
            let (pcm, _, present) = route(&source, block);
            assert_eq!(pcm.len(), 2 * FRAMES);
            assert_eq!(present, block.is_some());
            assert!(pcm.chunks_exact(2).all(|frame| frame == expected));
        }
    }

    #[test]
    fn missing_selected_channel_never_substitutes_another_input() {
        let source = Source {
            left: 2,
            ..Source::default()
        };
        let input = Block {
            channels: 2,
            samples: vec![1.; FRAMES * 2],
        };
        let (pcm, levels, present) = route(&source, Some(&input));
        assert!(!present);
        assert_eq!(levels, (0., 0.));
        assert_eq!(pcm, vec![0.; FRAMES]);
    }
}
