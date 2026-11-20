use crate::{
    Destination, Source,
    capture::{Capture, Reader},
    output::{self, Output, OutputStatus, StreamTarget},
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
pub(super) const BLOCK_DURATION: Duration = Duration::from_millis(10);
const FRAMES: usize = RATE * BLOCK_DURATION.as_millis() as usize / 1000;
type Devices = Vec<(String, String, usize)>;

struct Stream {
    name: String,
    target: Option<StreamTarget>,
    output: Result<Output, String>,
}

impl Stream {
    fn active(&self) -> bool {
        self.output.as_ref().is_ok_and(|output| !output.finished())
    }

    fn uses(&self, target: &StreamTarget) -> bool {
        self.target.as_ref() == Some(target) && self.active()
    }
}

#[derive(Default)]
struct Session {
    streams: Vec<Stream>,
    stream_attempt: u64,
    recorder: Option<Output>,
    record_attempt: u64,
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
            self.hold_remaining = self.hold_remaining.saturating_sub(BLOCK_DURATION);
        }
    }
}

impl Session {
    fn stop_streams(&mut self, stopping: &mut Vec<Stream>) {
        for mut stream in self.streams.drain(..) {
            if let Ok(output) = &mut stream.output
                && !output.finished()
            {
                output.stop();
                stopping.push(stream);
            }
        }
    }
}

#[derive(Default)]
struct Mixer {
    sources: Vec<Source>,
    sessions: HashMap<u64, Session>,
    stopping_streams: Vec<Stream>,
    finalizing: Vec<(String, Output)>,
}

#[derive(Default)]
struct Inputs {
    devices: Devices,
    readers: HashMap<String, Reader>,
    issues: HashMap<String, &'static str>,
    scan_issue: Option<&'static str>,
}

pub struct Engine {
    mixer: Arc<Mutex<Mixer>>,
    inputs: Arc<Mutex<Inputs>>,
    stop: Arc<AtomicBool>,
    rescan: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

impl Engine {
    pub fn new() -> Self {
        let mixer = Arc::new(Mutex::new(Mixer::default()));
        let inputs = Arc::new(Mutex::new(Inputs::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let rescan = Arc::new(AtomicBool::new(false));
        let manager = {
            let mixer = Arc::clone(&mixer);
            let inputs = Arc::clone(&inputs);
            let stop = Arc::clone(&stop);
            let rescan = Arc::clone(&rescan);
            thread::spawn(move || {
                let mut capture = Capture::new();
                while !stop.load(Ordering::Acquire) {
                    if rescan.swap(false, Ordering::AcqRel) {
                        capture.force_scan();
                    }
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
                        issues: capture.issues.clone(),
                        scan_issue: capture.scan_issue,
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
                    next += BLOCK_DURATION;
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
            rescan,
            workers: vec![clock, manager],
        }
    }

    pub fn refresh_devices(&self) {
        self.rescan.store(true, Ordering::Release);
    }

    pub fn devices(&self) -> Devices {
        self.inputs.lock().unwrap().devices.clone()
    }

    pub fn input_hint(&self, source: &Source) -> &'static str {
        let inputs = self.inputs.lock().unwrap();
        if let Some(reason) = inputs
            .issues
            .get(&source.device)
            .copied()
            .or(inputs.scan_issue)
        {
            return reason;
        }
        if source.device.is_empty() {
            return "Välj en ljudingång i ljudkällans inställningar.";
        }
        if let Some((_, _, channels)) = inputs
            .devices
            .iter()
            .find(|(id, _, _)| id == &source.device)
            && (source.left >= *channels || source.right.is_some_and(|right| right >= *channels))
        {
            return "En vald kanal saknas på ljudenheten. Välj en tillgänglig kanal i ljudkällans inställningar.";
        }
        "Väntar på ljudingången. Kontrollera att rätt ljudenhet är ansluten och vald."
    }
    pub fn update(&self, sources: &[Source]) {
        let mut mixer = self.mixer.lock().unwrap();
        let Mixer {
            sessions,
            stopping_streams,
            finalizing,
            sources: previous,
            ..
        } = &mut *mixer;
        sessions.retain(|id, session| {
            let Some(source) = sources.iter().find(|source| source.id == *id) else {
                session.stop_streams(stopping_streams);
                if let Some(mut recorder) = session.recorder.take() {
                    recorder.stop();
                    let name = previous
                        .iter()
                        .find(|s| s.id == *id)
                        .map_or("Namnlös ljudkälla", Source::display_name);
                    finalizing.push((name.to_owned(), recorder));
                }
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
            .is_some_and(|s| s.streams.iter().any(Stream::active))
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
            status.stream_attempt = session.stream_attempt;
            status.record_attempt = session.record_attempt;
            for (index, stream) in session.streams.iter().enumerate() {
                let (sending, detail, audio_gap, encoded) = match &stream.output {
                    Ok(output) => {
                        let encoded = output.encoded_time();
                        (
                            !output.finished() && !encoded.is_zero(),
                            output.status(),
                            output.dropped_audio(),
                            encoded,
                        )
                    }
                    Err(error) => (
                        false,
                        OutputStatus::Problem(error.clone()),
                        false,
                        Duration::ZERO,
                    ),
                };
                status.connected += usize::from(sending);
                if sending {
                    status.stream_elapsed = status.stream_elapsed.max(encoded);
                }
                let name = if stream.name.is_empty() {
                    format!("Sändare {}", index + 1)
                } else {
                    stream.name.clone()
                };
                status.targets.push(TargetStatus {
                    name,
                    detail,
                    audio_gap,
                    sending,
                });
            }
            if let Some(recorder) = &session.recorder {
                status.elapsed = recorder.encoded_time();
                if let OutputStatus::Progress(detail) = recorder.status() {
                    status.record_detail = detail;
                }
                status.record_audio_gap = recorder.dropped_audio();
            }
        }
        status
    }

    pub fn completed_recordings(&self) -> Vec<(String, Result<std::path::PathBuf, String>)> {
        let mut mixer = self.mixer.lock().unwrap();
        let mut results = Vec::new();
        let Mixer {
            sources,
            sessions,
            finalizing,
            ..
        } = &mut *mixer;
        for source in sources {
            if let Some(recorder) = sessions
                .get_mut(&source.id)
                .and_then(|session| session.recorder.take_if(|output| output.finished()))
            {
                results.push((
                    source.display_name().to_owned(),
                    recorder.recording_result(),
                ));
            }
        }
        finalizing.retain(|(name, output)| {
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
        self.mixer
            .lock()
            .unwrap()
            .set_stream(source, destinations, start);
    }
    pub fn set_record(&self, source: &Source, start: bool) {
        let mut m = self.mixer.lock().unwrap();
        let Mixer {
            sessions,
            finalizing,
            ..
        } = &mut *m;
        let s = sessions.entry(source.id).or_default();
        if let Some(mut recorder) = s.recorder.take_if(|r| !start || r.finished()) {
            recorder.stop();
            finalizing.push((source.display_name().to_owned(), recorder));
        }
        if start && s.recorder.is_none() {
            s.record_attempt += 1;
            s.recorder = Some(Output::record(
                &source.recording,
                &source.name,
                channel_count(source),
            ));
        }
    }

    pub fn shutdown(&mut self) -> Vec<String> {
        if self.stop.swap(true, Ordering::AcqRel) {
            return Vec::new();
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
        let mut mixer = self.mixer.lock().unwrap();
        let Mixer {
            sources,
            sessions,
            finalizing,
            ..
        } = &mut *mixer;
        for source in sources {
            if let Some(mut recorder) = sessions.get_mut(&source.id).and_then(|s| s.recorder.take())
            {
                recorder.stop();
                finalizing.push((source.display_name().to_owned(), recorder));
            }
        }
        sessions.clear();
        let recordings = std::mem::take(finalizing);
        drop(mixer);
        output::wait_for_shutdown();
        recordings
            .into_iter()
            .filter_map(|(name, output)| {
                output
                    .recording_result()
                    .err()
                    .map(|err| format!("{name} · {err}"))
            })
            .collect()
    }
}

#[derive(Default)]
pub struct LiveStatus {
    pub connected: usize,
    pub stream_attempt: u64,
    pub stream_elapsed: Duration,
    pub targets: Vec<TargetStatus>,
    pub record_attempt: u64,
    pub elapsed: Duration,
    pub record_detail: String,
    pub record_audio_gap: bool,
}

pub struct TargetStatus {
    pub name: String,
    pub detail: OutputStatus,
    pub sending: bool,
    pub audio_gap: bool,
}

impl Drop for Engine {
    fn drop(&mut self) {
        self.shutdown();
    }
}

struct Block {
    channels: usize,
    samples: Vec<f32>,
}

fn read_inputs(
    readers: &HashMap<String, Reader>,
    converters: &mut HashMap<String, Converter>,
) -> HashMap<String, Block> {
    converters.retain(|id, _| readers.contains_key(id));
    readers
        .iter()
        .filter_map(|(id, reader)| {
            converters
                .entry(id.clone())
                .or_default()
                .pull(reader)
                .map(|block| (id.clone(), block))
        })
        .collect()
}

fn channel_count(source: &Source) -> u16 {
    if source.right.is_some() { 2 } else { 1 }
}

impl Mixer {
    fn set_stream(&mut self, source: &Source, destinations: &[Destination], start: bool) {
        let session = self.sessions.entry(source.id).or_default();
        if !start {
            session.stop_streams(&mut self.stopping_streams);
            return;
        }
        if session.streams.iter().any(Stream::active) {
            return;
        }
        session.stream_attempt += 1;

        let mut streams = Vec::<Stream>::new();
        for dest in source.resolve_destinations(destinations) {
            let (target, output) = match StreamTarget::parse(dest) {
                Ok((target, tls)) => {
                    let output = if streams.iter().any(|stream| stream.uses(&target)) {
                        Err("Samma monteringspunkt är redan vald för den här ljudkällan.".into())
                    } else if let Some(error) = self.stream_conflict(&target) {
                        Err(error)
                    } else {
                        Output::stream(dest, channel_count(source), &target, tls)
                            .map_err(str::to_owned)
                    };
                    (Some(target), output)
                }
                Err(error) => (None, Err(error.into())),
            };
            streams.push(Stream {
                name: dest.name.trim().to_owned(),
                target,
                output,
            });
        }
        self.sessions.entry(source.id).or_default().streams = streams;
    }

    fn stream_conflict(&self, target: &StreamTarget) -> Option<String> {
        if let Some((&id, _)) = self
            .sessions
            .iter()
            .find(|(_, session)| session.streams.iter().any(|stream| stream.uses(target)))
        {
            let name = self
                .sources
                .iter()
                .find(|source| source.id == id)
                .map(|source| source.name.trim())
                .filter(|name| !name.is_empty());
            return Some(match name {
                Some(name) => format!("Destinationen används redan av ljudkällan ”{name}”."),
                None => "Destinationen används redan av en annan ljudkälla.".into(),
            });
        }
        if self
            .stopping_streams
            .iter()
            .any(|stream| stream.uses(target))
        {
            return Some(
                "Sändningen till destinationen håller på att stoppas. Försök igen om en stund."
                    .into(),
            );
        }
        None
    }

    fn render(&mut self, inputs: &HashMap<String, Block>) {
        self.stopping_streams.retain(Stream::active);
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
                .filter_map(|stream| stream.output.as_ref().ok())
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
    use crate::capture::tests::{feed, synthetic_reader};

    fn test_engine() -> Engine {
        Engine {
            mixer: Arc::new(Mutex::new(Mixer::default())),
            inputs: Arc::new(Mutex::new(Inputs::default())),
            stop: Arc::new(AtomicBool::new(false)),
            rescan: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
        }
    }

    #[test]
    fn shutdown_reports_recording_failures_after_the_main_window_closes() {
        let mut engine = test_engine();
        let mut source = Source {
            name: "Stora salen".into(),
            ..Default::default()
        };
        source.recording.format = "okänt".into();
        engine.update(std::slice::from_ref(&source));
        engine.set_record(&source, true);
        let errors = engine.shutdown();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Stora salen") && errors[0].contains("Lokal inspelning"));
        assert!(engine.shutdown().is_empty());
    }

    #[test]
    fn recording_failures_survive_a_retry_and_are_delivered_once_per_source() {
        let engine = test_engine();
        let mut first = Source {
            id: 1,
            name: "Sal A".into(),
            ..Default::default()
        };
        first.recording.format = "okänt".into();
        let second = Source {
            id: 2,
            name: "Sal B".into(),
            ..first.clone()
        };
        engine.update(&[first.clone(), second.clone()]);
        engine.set_record(&first, true);
        engine.set_record(&second, true);
        let wait_for_recorders = || {
            let deadline = Instant::now() + Duration::from_secs(5);
            while engine.recording(1) || engine.recording(2) {
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(5));
            }
        };
        wait_for_recorders();
        engine.set_record(&first, true);
        wait_for_recorders();
        let mut results = engine.completed_recordings();
        results.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            results
                .iter()
                .map(|(name, _)| name.as_str())
                .collect::<Vec<_>>(),
            ["Sal A", "Sal A", "Sal B"]
        );
        assert!(results.iter().all(|(_, result)| {
            result
                .as_ref()
                .is_err_and(|err| err.contains("Lokal inspelning"))
        }));
        assert!(engine.completed_recordings().is_empty());
    }

    #[test]
    fn converter_preserves_fractional_phase_and_channels_across_blocks() {
        let input_rate = 32_003;
        let reader = synthetic_reader(2, input_rate);
        let samples: Vec<_> = (0..2500)
            .flat_map(|index| {
                let value = index as f32 / 4096.;
                [value, -value]
            })
            .collect();
        feed(&reader, &samples);

        let mut converter = Converter::default();
        for block_index in 0..2 {
            let block = converter.pull(&reader).unwrap();
            assert_eq!(block.channels, 2);
            assert_eq!(block.samples.len(), FRAMES * 2);
            for (index, frame) in block.samples.chunks_exact(2).enumerate() {
                let output_index = block_index * FRAMES + index;
                let expected = output_index as f32 * input_rate as f32 / RATE as f32 / 4096.;
                assert!((frame[0] - expected).abs() < 1e-6, "frame {output_index}");
                assert_eq!(frame[1], -frame[0]);
            }
        }
    }

    #[test]
    fn converter_pads_underrun_and_rebuffers_before_resuming() {
        let reader = synthetic_reader(1, RATE as u32);
        let mut converter = Converter::default();
        feed(&reader, &[0.75; 1439]);
        assert!(converter.pull(&reader).is_none());
        feed(&reader, &[0.75]);
        assert_eq!(converter.pull(&reader).unwrap().samples, vec![0.75; FRAMES]);

        // Simulate the input drying up after the converter's retained lookahead.
        reader.read(usize::MAX).unwrap();
        let block = converter.pull(&reader).unwrap();
        assert_eq!(block.samples.len(), FRAMES);
        assert_eq!(block.samples[0], 0.75);
        assert!(block.samples[2..].iter().all(|&sample| sample == 0.));

        feed(&reader, &[-0.5; 1439]);
        assert!(converter.pull(&reader).is_none());
        feed(&reader, &[-0.5]);
        assert_eq!(converter.pull(&reader).unwrap().samples, vec![-0.5; FRAMES]);
    }

    #[test]
    fn device_removal_discards_converter_history_before_replug() {
        let reader = synthetic_reader(2, RATE as u32);
        let mut readers = HashMap::from([("device".into(), reader.clone())]);
        let mut converters = HashMap::new();
        feed(&reader, &[0.75; 1440 * 2]);
        assert!(read_inputs(&readers, &mut converters).contains_key("device"));

        readers.clear();
        assert!(read_inputs(&readers, &mut converters).is_empty());

        let replacement = synthetic_reader(2, RATE as u32);
        feed(&replacement, &[-0.5; 1439 * 2]);
        readers.insert("device".into(), replacement.clone());
        assert!(read_inputs(&readers, &mut converters).is_empty());
        feed(&replacement, &[-0.5, -0.5]);
        assert_eq!(
            read_inputs(&readers, &mut converters)["device"].samples,
            vec![-0.5; FRAMES * 2]
        );
    }

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
    fn competing_sources_can_only_start_on_unoccupied_mounts() {
        let first = Source {
            name: "Mikrofon".into(),
            destination_ids: vec![1],
            ..Source::default()
        };
        let second = Source {
            id: 2,
            destination_ids: vec![2, 3],
            ..Source::default()
        };
        let shared = Destination {
            id: 1,
            server: "http://127.0.0.1".into(),
            ..Destination::default()
        };
        let duplicate = Destination {
            id: 2,
            name: "Huvudsändning".into(),
            server: " http://127.0.0.1:80/ignored?setting=ignored ".into(),
            mount: " //live.opus ".into(),
            user: "another-user".into(),
            password: "another-password".into(),
            codec: "MP3".into(),
            ..shared.clone()
        };
        let separate = Destination {
            id: 3,
            mount: "/other.opus".into(),
            ..shared.clone()
        };
        let destinations = [shared, duplicate, separate];
        let engine = test_engine();
        engine.update(&[first.clone(), second.clone()]);

        engine.set_stream(&first, &destinations, true);
        engine.set_stream(&second, &destinations, true);

        assert!(engine.streaming(first.id));
        assert!(engine.streaming(second.id));
        assert!(engine.live_status(second.id).targets.iter().any(|target| {
            target.name == "Huvudsändning" && matches!(&target.detail,
                OutputStatus::Problem(reason) if reason == "Destinationen används redan av ljudkällan ”Mikrofon”.")
        }));
        {
            let mixer = engine.mixer.lock().unwrap();
            let streams = &mixer.sessions[&second.id].streams;
            assert!(streams[0].output.is_err());
            assert!(streams[1].output.is_ok());
        }

        engine.set_stream(&first, &destinations, false);
        engine.set_stream(&second, &destinations, false);
        let deadline = Instant::now() + Duration::from_secs(2);
        while engine
            .mixer
            .lock()
            .unwrap()
            .stopping_streams
            .iter()
            .any(Stream::active)
        {
            assert!(Instant::now() < deadline, "stream workers did not stop");
            thread::sleep(BLOCK_DURATION);
        }
        engine.set_stream(&second, &destinations, true);
        let mixer = engine.mixer.lock().unwrap();
        assert!(mixer.sessions[&second.id].streams[0].output.is_ok());
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
