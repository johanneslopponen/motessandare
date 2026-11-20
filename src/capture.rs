use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{
    Device, ErrorKind, FromSample, SampleFormat, SizedSample, Stream, SupportedStreamConfig,
};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const STALL_TIMEOUT: Duration = Duration::from_secs(3);
const BUFFER_MILLISECONDS: usize = 250;

pub struct Capture {
    host: cpal::Host,
    devices: BTreeMap<String, InputDevice>,
    streams: BTreeMap<String, InputStream>,
    last_scan: Option<Instant>,
    retry_after: BTreeMap<String, Instant>,
    pub issues: HashMap<String, &'static str>,
    pub scan_issue: Option<&'static str>,
}

struct InputDevice {
    device: Device,
    name: String,
    configs: Vec<SupportedStreamConfig>,
}

struct InputStream {
    _stream: Stream,
    reader: Reader,
    failed: Arc<AtomicBool>,
    received_samples: Arc<AtomicBool>,
    last_data: Instant,
}

#[derive(Clone)]
pub struct Reader {
    ring: Arc<AudioRing>,
    channels: usize,
    rate: u32,
}

struct AudioRing {
    samples: Box<[AtomicU32]>,
    channels: usize,
    capacity: usize,
    read_at: AtomicUsize,
    written_at: AtomicUsize,
    // Readers may be cloned, so serialize their consumption. The producer never locks.
    reader_lock: Mutex<()>,
}

impl Capture {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
            devices: BTreeMap::new(),
            streams: BTreeMap::new(),
            last_scan: None,
            retry_after: BTreeMap::new(),
            issues: HashMap::new(),
            scan_issue: None,
        }
    }

    pub fn devices(&self) -> Vec<(String, String, usize)> {
        self.devices
            .iter()
            .map(|(id, device)| {
                let channels = self.streams.get(id).map_or_else(
                    || usize::from(device.configs[0].channels()),
                    |stream| stream.reader.channels,
                );
                (id.clone(), device.name.clone(), channels)
            })
            .collect()
    }

    pub fn tick(&mut self, wanted: &[String]) {
        let now = Instant::now();
        self.issues.retain(|id, _| wanted.contains(id));
        self.streams.retain(|id, stream| {
            if stream.received_samples.swap(false, Ordering::Relaxed) {
                stream.last_data = now;
                self.issues.remove(id);
            }
            if !wanted.contains(id) {
                return false;
            }
            if stream.failed.load(Ordering::Relaxed)
                || now.duration_since(stream.last_data) >= STALL_TIMEOUT
            {
                self.issues.insert(
                    id.clone(),
                    "Ljudingången slutade lämna ljud. Appen försöker ansluta igen.",
                );
                self.retry_after.insert(id.clone(), now + POLL_INTERVAL);
                return false;
            }
            true
        });
        self.retry_after
            .retain(|id, until| *until > now && wanted.contains(id));

        if self
            .last_scan
            .is_none_or(|last| now.duration_since(last) >= POLL_INTERVAL)
        {
            self.last_scan = Some(now);
            self.scan();
        }

        for id in wanted {
            if self.streams.contains_key(id) || self.retry_after.contains_key(id) {
                continue;
            }
            let Some(input) = self.devices.get(id) else {
                continue;
            };
            match InputStream::open(input) {
                Ok(stream) => {
                    self.streams.insert(id.clone(), stream);
                }
                Err(error) => {
                    self.issues
                        .insert(id.clone(), input_error_hint(error.kind()));
                    self.retry_after.insert(id.clone(), now + STALL_TIMEOUT);
                }
            }
        }
    }

    fn scan(&mut self) {
        let devices = match self.host.devices() {
            Ok(devices) => devices,
            Err(error) => {
                self.scan_issue = Some(input_error_hint(error.kind()));
                return;
            }
        };
        self.scan_issue = None;

        let mut found = BTreeMap::new();
        for device in devices {
            let Ok(id) = device.id() else {
                continue;
            };
            let id = id.to_string();
            if found.contains_key(&id) {
                continue;
            }

            // ALSA may refuse a second open for probing while we already capture.
            if self.streams.contains_key(&id)
                && let Some(input) = self.devices.remove(&id)
            {
                found.insert(id, input);
                continue;
            }

            if !device.supports_input() {
                continue;
            }
            let configs = input_configs(&device);
            if configs.is_empty() {
                continue;
            }
            let name = device
                .description()
                .map(|description| description.name().to_owned())
                .unwrap_or_else(|_| id.clone());
            found.insert(
                id,
                InputDevice {
                    device,
                    name,
                    configs,
                },
            );
        }

        self.streams.retain(|id, _| found.contains_key(id));
        self.issues.retain(|id, _| found.contains_key(id));
        self.devices = found;
    }

    pub fn readers(&self) -> HashMap<String, Reader> {
        self.streams
            .iter()
            .map(|(id, stream)| (id.clone(), stream.reader.clone()))
            .collect()
    }
}

fn input_error_hint(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::PermissionDenied => {
            "Ge Mötessändare åtkomst till mikrofonen i datorns integritetsinställningar."
        }
        ErrorKind::DeviceBusy => "Ljudingången är upptagen. Stäng andra program som använder den.",
        ErrorKind::UnsupportedConfig | ErrorKind::InvalidInput => {
            "Ljudenhetens format stöds inte. Kontrollera enhetens ljudinställningar."
        }
        ErrorKind::DeviceNotAvailable => {
            "Ljudenheten är frånkopplad. Anslut den igen eller välj en annan ljudingång."
        }
        _ => {
            "Datorns ljudingång kunde inte öppnas. Kontrollera enhetens anslutning och datorns ljudinställningar."
        }
    }
}

impl Reader {
    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn queued_frames(&self) -> Option<usize> {
        let _reader = self.ring.reader_lock.lock().ok()?;
        let read = self.ring.read_at.load(Ordering::Relaxed);
        let written = self.ring.written_at.load(Ordering::Acquire);
        Some(written.wrapping_sub(read))
    }

    pub fn read(&self, frames: usize) -> Option<(Vec<f32>, usize, u32)> {
        let samples = self.ring.read(frames)?;
        Some((samples, self.channels, self.rate))
    }
}

impl AudioRing {
    fn new(channels: usize, minimum_frames: usize) -> Self {
        let capacity = minimum_frames.next_power_of_two();
        Self {
            samples: (0..capacity * channels)
                .map(|_| AtomicU32::new(0))
                .collect(),
            channels,
            capacity,
            read_at: AtomicUsize::new(0),
            written_at: AtomicUsize::new(0),
            reader_lock: Mutex::new(()),
        }
    }

    // Only the CPAL data callback calls push. Release/Acquire transfers the complete frames.
    fn push<T: Copy>(&self, input: &[T])
    where
        f32: FromSample<T>,
    {
        let written = self.written_at.load(Ordering::Relaxed);
        let read = self.read_at.load(Ordering::Acquire);
        let available = self.capacity - written.wrapping_sub(read);
        let frames = (input.len() / self.channels).min(available);
        let start = written & (self.capacity - 1);
        let first = frames.min(self.capacity - start) * self.channels;
        let offset = start * self.channels;
        let last = frames * self.channels - first;
        for (slot, &sample) in self.samples[offset..offset + first]
            .iter()
            .chain(&self.samples[..last])
            .zip(input)
        {
            slot.store(f32::from_sample_(sample).to_bits(), Ordering::Relaxed);
        }
        self.written_at
            .store(written.wrapping_add(frames), Ordering::Release);
    }

    fn read(&self, frames: usize) -> Option<Vec<f32>> {
        let _reader = self.reader_lock.lock().ok()?;
        let read = self.read_at.load(Ordering::Relaxed);
        let written = self.written_at.load(Ordering::Acquire);
        let frames = frames.min(written.wrapping_sub(read));
        let start = read & (self.capacity - 1);
        let first = frames.min(self.capacity - start) * self.channels;
        let offset = start * self.channels;
        let last = frames * self.channels - first;
        let samples = self.samples[offset..offset + first]
            .iter()
            .chain(&self.samples[..last])
            .map(|slot| f32::from_bits(slot.load(Ordering::Relaxed)))
            .collect();
        self.read_at
            .store(read.wrapping_add(frames), Ordering::Release);
        Some(samples)
    }
}

fn input_configs(device: &Device) -> Vec<SupportedStreamConfig> {
    let default = device.default_input_config().ok();
    let preferred_rate = default
        .as_ref()
        .map_or(48_000, SupportedStreamConfig::sample_rate);
    let mut configs = Vec::new();
    if let Some(config) =
        default.filter(|config| config.channels() > 0 && pcm(config.sample_format()))
    {
        configs.push(config);
    }
    if let Ok(ranges) = device.supported_input_configs() {
        configs.extend(ranges.filter_map(|range| {
            if range.channels() == 0 || !pcm(range.sample_format()) {
                return None;
            }
            let rate = preferred_rate.clamp(range.min_sample_rate(), range.max_sample_rate());
            range.try_with_sample_rate(rate)
        }));
    }

    // Preserve all hardware channels first, and use the device's native rate when possible.
    configs.sort_by_key(|config| {
        (
            std::cmp::Reverse(config.channels()),
            config.sample_rate().abs_diff(preferred_rate),
        )
    });
    configs.dedup();
    configs
}

fn pcm(format: SampleFormat) -> bool {
    matches!(
        format,
        SampleFormat::I8
            | SampleFormat::I16
            | SampleFormat::I24
            | SampleFormat::I32
            | SampleFormat::I64
            | SampleFormat::U8
            | SampleFormat::U16
            | SampleFormat::U24
            | SampleFormat::U32
            | SampleFormat::U64
            | SampleFormat::F32
            | SampleFormat::F64
    )
}

impl InputStream {
    fn open(input: &InputDevice) -> Result<Self, cpal::Error> {
        let mut error = cpal::Error::from(ErrorKind::UnsupportedConfig);
        for config in &input.configs {
            match Self::build(&input.device, config) {
                Ok(stream) => return Ok(stream),
                Err(cause) => {
                    let try_next = matches!(
                        cause.kind(),
                        ErrorKind::UnsupportedConfig | ErrorKind::InvalidInput
                    );
                    error = cause;
                    if !try_next {
                        break;
                    }
                }
            }
        }
        Err(error)
    }

    fn build(device: &Device, config: &SupportedStreamConfig) -> Result<Self, cpal::Error> {
        let channels = usize::from(config.channels());
        let rate = config.sample_rate();
        let minimum_frames = ((rate as usize / 1_000) * BUFFER_MILLISECONDS).max(1);
        let ring = Arc::new(AudioRing::new(channels, minimum_frames));
        let failed = Arc::new(AtomicBool::new(false));
        let received_samples = Arc::new(AtomicBool::new(false));

        let error_flag = Arc::clone(&failed);
        let on_error = move |error: cpal::Error| {
            if !matches!(error.kind(), ErrorKind::Xrun | ErrorKind::RealtimeDenied) {
                error_flag.store(true, Ordering::Relaxed);
            }
        };

        macro_rules! stream {
            ($sample:ty) => {
                device.build_input_stream(
                    config.config(),
                    input_callback::<$sample>(Arc::clone(&ring), Arc::clone(&received_samples)),
                    on_error,
                    None,
                )
            };
        }

        let stream = match config.sample_format() {
            SampleFormat::I8 => stream!(i8),
            SampleFormat::I16 => stream!(i16),
            SampleFormat::I24 => stream!(cpal::I24),
            SampleFormat::I32 => stream!(i32),
            SampleFormat::I64 => stream!(i64),
            SampleFormat::U8 => stream!(u8),
            SampleFormat::U16 => stream!(u16),
            SampleFormat::U24 => stream!(cpal::U24),
            SampleFormat::U32 => stream!(u32),
            SampleFormat::U64 => stream!(u64),
            SampleFormat::F32 => stream!(f32),
            SampleFormat::F64 => stream!(f64),
            _ => return Err(ErrorKind::UnsupportedConfig.into()),
        }?;
        stream.play()?;

        Ok(Self {
            _stream: stream,
            reader: Reader {
                ring,
                channels,
                rate,
            },
            failed,
            received_samples,
            last_data: Instant::now(),
        })
    }
}

fn input_callback<T>(
    ring: Arc<AudioRing>,
    received_samples: Arc<AtomicBool>,
) -> impl FnMut(&[T], &cpal::InputCallbackInfo) + Send + 'static
where
    T: SizedSample,
    f32: FromSample<T>,
{
    move |input, _| {
        if input.len() < ring.channels {
            return;
        }
        received_samples.store(true, Ordering::Relaxed);
        ring.push(input);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{AudioRing, Reader};
    use std::sync::Arc;

    pub(crate) fn synthetic_reader(channels: usize, rate: u32) -> Reader {
        Reader {
            ring: Arc::new(AudioRing::new(channels, 8192)),
            channels,
            rate,
        }
    }

    pub(crate) fn feed(reader: &Reader, samples: &[f32]) {
        reader.ring.push(samples);
    }

    #[test]
    fn cloned_readers_share_consumption_and_pcm_is_normalized() {
        let reader = synthetic_reader(2, 48_000);
        reader.ring.push(&[i16::MIN, 0, 16_384, -16_384]);
        let clone = reader.clone();

        assert_eq!(reader.queued_frames(), Some(2));
        assert_eq!(clone.read(1), Some((vec![-1.0, 0.0], 2, 48_000)));
        assert_eq!(reader.queued_frames(), Some(1));
        assert_eq!(reader.read(8), Some((vec![0.5, -0.5], 2, 48_000)));
        assert_eq!(clone.queued_frames(), Some(0));
    }

    #[test]
    fn overflow_preserves_existing_frames_and_wraps_in_channel_order() {
        let ring = AudioRing::new(2, 4);
        ring.push(&[1f32, 10., 2., 20., 3., 30.]);
        assert_eq!(ring.read(2).unwrap(), [1., 10., 2., 20.]);
        ring.push(&[4f32, 40., 5., 50., 6., 60., 7., 70.]);
        assert_eq!(ring.read(8).unwrap(), [3., 30., 4., 40., 5., 50., 6., 60.]);
        ring.push(&[8f32, 80., 9.]);
        assert_eq!(ring.read(8).unwrap(), [8., 80.]);
    }
}
