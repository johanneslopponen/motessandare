use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{
            AtomicBool, AtomicI64, AtomicUsize,
            Ordering::{Acquire, Relaxed, Release},
        },
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use url::Url;

use crate::{Destination, Recording};

const POLL: Duration = Duration::from_millis(50);
const RETRY_DELAY: Duration = Duration::from_secs(3);
const STALL_TIMEOUT: Duration = Duration::from_secs(12);
const FINALIZE_TIMEOUT: Duration = Duration::from_secs(8);

static ACTIVE_WORKERS: AtomicUsize = AtomicUsize::new(0);

struct WorkerGuard(Arc<State>);

impl Drop for WorkerGuard {
    fn drop(&mut self) {
        self.0.finished.store(true, Release);
        ACTIVE_WORKERS.fetch_sub(1, Relaxed);
    }
}

pub struct Output {
    sender: Option<SyncSender<Vec<f32>>>,
    state: Arc<State>,
}

struct State {
    status: Mutex<String>,
    stop: AtomicBool,
    finished: AtomicBool,
    dropped: AtomicUsize,
    encoded_us: AtomicI64,
    saved_file: Mutex<Option<PathBuf>>,
}

impl State {
    fn set_status(&self, text: impl Into<String>) {
        *self.status.lock().unwrap() = text.into();
    }
}

impl Output {
    pub fn stream(dest: &Destination, channels: u16) -> Self {
        let setup = stream_command(dest, channels);
        Self::start(32, move |receiver, state| match setup {
            Ok(command) => run_stream(command, receiver, &state),
            Err(error) => state.set_status(error),
        })
    }

    pub fn record(settings: &Recording, name: &str, channels: u16) -> Self {
        let settings = settings.clone();
        let name = name.to_owned();
        Self::start(128, move |receiver, state| {
            run_record(&settings, &name, channels, receiver, &state);
        })
    }

    fn start(
        queue_capacity: usize,
        work: impl FnOnce(Receiver<Vec<f32>>, Arc<State>) + Send + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::sync_channel(queue_capacity);
        let state = Arc::new(State {
            status: Mutex::new("Startar…".into()),
            stop: AtomicBool::new(false),
            finished: AtomicBool::new(false),
            dropped: AtomicUsize::new(0),
            encoded_us: AtomicI64::new(0),
            saved_file: Mutex::new(None),
        });
        let worker_state = Arc::clone(&state);
        ACTIVE_WORKERS.fetch_add(1, Relaxed);
        let guard = WorkerGuard(Arc::clone(&state));
        let worker = thread::Builder::new()
            .name("ffmpeg-output".into())
            .spawn(move || {
                let _guard = guard;
                work(receiver, worker_state);
            });
        if let Err(error) = worker {
            state.set_status(format!("Kunde inte starta FFmpeg-arbetaren: {error}"));
        }
        Self {
            sender: Some(sender),
            state,
        }
    }

    pub fn push(&self, pcm: &[f32]) {
        if let Some(sender) = &self.sender
            && !pcm.is_empty()
            && matches!(sender.try_send(pcm.to_vec()), Err(TrySendError::Full(_)))
        {
            self.state.dropped.fetch_add(1, Relaxed);
        }
    }

    pub fn status(&self) -> String {
        let status = self.state.status.lock().unwrap();
        let dropped = self.state.dropped.load(Relaxed);
        if dropped > 0 {
            return format!("{status} · {dropped} tappade ljudblock");
        }
        status.clone()
    }

    pub fn encoded_time(&self) -> Duration {
        Duration::from_micros(self.state.encoded_us.load(Relaxed) as u64)
    }

    pub fn stop(&mut self) {
        self.state.stop.store(true, Relaxed);
        self.sender.take();
    }

    pub fn recording_result(&self) -> Result<PathBuf, String> {
        self.state.saved_file.lock().unwrap().clone().ok_or_else(|| self.status())
    }

    pub fn finished(&self) -> bool {
        self.state.finished.load(Acquire)
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Call after dropping the outputs, so FFmpeg can write recording trailers on exit.
pub fn wait_for_shutdown() {
    let deadline = Instant::now() + FINALIZE_TIMEOUT + Duration::from_secs(2);
    while ACTIVE_WORKERS.load(Relaxed) > 0 && Instant::now() < deadline {
        thread::sleep(POLL);
    }
}

fn ffmpeg_path() -> PathBuf {
    if let Ok(executable) = std::env::current_exe()
        && let Some(executable_dir) = executable.parent()
    {
        let binary = if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        };
        let adjacent = executable_dir.join(binary);
        if adjacent.is_file() {
            return adjacent;
        }
        #[cfg(target_os = "macos")]
        if let Some(contents) = executable_dir.parent() {
            let resource = contents.join("Resources").join(binary);
            if resource.is_file() {
                return resource;
            }
        }
    }
    PathBuf::from("ffmpeg")
}

fn ffmpeg(channels: u16) -> Command {
    let mut command = Command::new(ffmpeg_path());
    command
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-nostdin",
            "-progress",
            "pipe:1",
            "-f",
            "f32le",
            "-probesize",
            "32",
            "-analyzeduration",
            "0",
            "-ar",
            "48000",
            "-ac",
            &channels.to_string(),
            "-i",
            "pipe:0",
            "-map",
            "0:a:0",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        // FFmpeg includes connection credentials in its diagnostics.
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command
}

fn stream_command(dest: &Destination, channels: u16) -> Result<Command, &'static str> {
    let mut server = Url::parse(dest.server.trim()).map_err(|_| "Ogiltig serveradress")?;
    let tls = match server.scheme() {
        "http" => false,
        "https" => true,
        _ => return Err("Serveradressen måste börja med http:// eller https://"),
    };
    let port = server.port_or_known_default().ok_or("Ogiltig serverport")?;
    let host = server
        .host()
        .ok_or("Serveradressen saknar värdnamn")?
        .to_string();
    let mount = dest.mount.trim().trim_start_matches('/');
    if mount.is_empty() || mount.contains(['?', '#']) || mount.chars().any(char::is_control) {
        return Err("Ogiltig monteringspunkt");
    }
    server.set_path(&format!("/{mount}"));
    let user = dest.user.trim();
    if user.contains([':', '/', '?', '#', '@', '%']) || user.chars().any(char::is_whitespace) {
        return Err("Icecast-användarnamnet innehåller ogiltiga tecken");
    }
    let user = if user.is_empty() { "source" } else { user };
    let endpoint = format!("icecast://{user}@{host}:{port}{}", server.path());
    let (encoder, muxer, content_type) = match dest.codec.as_str() {
        "Opus" => ("libopus", "ogg", "application/ogg"),
        "MP3" => ("libmp3lame", "mp3", "audio/mpeg"),
        _ => return Err("Okänd sändningskodning"),
    };
    let mut command = ffmpeg(channels);
    if muxer == "ogg" {
        command.args(["-page_duration", "200000"]);
    }
    command.args([
        "-c:a",
        encoder,
        "-b:a",
        &format!("{}k", dest.bitrate),
        "-f",
        muxer,
        "-flush_packets",
        "1",
        "-rw_timeout",
        "8000000",
        "-tls",
        if tls { "1" } else { "0" },
        "-content_type",
        content_type,
        "-password",
        &dest.password,
        &endpoint,
    ]);
    Ok(command)
}

fn run_stream(mut command: Command, mut receiver: Receiver<Vec<f32>>, state: &State) {
    while !state.stop.load(Relaxed) {
        state.set_status("Ansluter…");
        let (returned, result) = encode(&mut command, receiver, state, "Sänder", true);
        receiver = returned;
        if state.stop.load(Relaxed) {
            return;
        }
        state.encoded_us.store(0, Relaxed);
        if let Err(error) = result {
            state.set_status(format!("{error} · försöker igen…"));
        } else {
            state.set_status("Anslutningen bröts · försöker igen…");
        }
        let deadline = Instant::now() + RETRY_DELAY;
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            if receiver.recv_timeout(remaining).is_err() {
                break;
            }
        }
    }
}

fn run_record(
    settings: &Recording,
    name: &str,
    channels: u16,
    receiver: Receiver<Vec<f32>>,
    state: &State,
) {
    let format = settings.format.as_str();
    let quality = settings.quality;
    let (encoder, muxer, extension) = match format {
        "WAV" => (
            if quality >= 24 {
                "pcm_s24le"
            } else {
                "pcm_s16le"
            },
            "wav",
            "wav",
        ),
        "FLAC" => ("flac", "flac", "flac"),
        "Opus" => ("libopus", "ogg", "opus"),
        "MP3" => ("libmp3lame", "mp3", "mp3"),
        _ => {
            state.set_status("Okänt inspelningsformat");
            return;
        }
    };
    let path = match reserve_recording_path(
        Path::new(&settings.folder),
        &settings.filename_format,
        name,
        extension,
    ) {
        Ok(path) => path,
        Err(error) => {
            state.set_status(format!("Kunde inte skapa inspelningsfilen: {error}"));
            return;
        }
    };
    let mut command = ffmpeg(channels);
    command.args(["-y", "-c:a", encoder]);
    if format == "FLAC" {
        command.args(["-sample_fmt", if quality >= 24 { "s32" } else { "s16" }]);
    }
    if matches!(format, "Opus" | "MP3") {
        command.args(["-b:a", &format!("{quality}k")]);
    }
    if format == "WAV" {
        command.args(["-rf64", "auto"]);
    }
    command.args(["-f", muxer]).arg(&path);
    let filename = path.file_name().unwrap().to_string_lossy();
    let (_, result) = encode(
        &mut command,
        receiver,
        state,
        &format!("Spelar in · {filename}"),
        false,
    );
    match result {
        Ok(()) => {
            state.set_status(format!("Sparad · {filename}"));
            *state.saved_file.lock().unwrap() = Some(path);
        }
        Err(error) => {
            if fs::metadata(&path).is_ok_and(|metadata| metadata.len() == 0) {
                let _ = fs::remove_file(&path);
            }
            state.set_status(format!("{error} · {filename}"));
        }
    }
}

#[derive(Clone, Copy)]
struct LocalDateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    unix: u64,
}

fn local_now() -> LocalDateTime {
    let unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    #[cfg(windows)]
    {
        #[repr(C)]
        struct WinSystemTime {
            year: u16,
            month: u16,
            day_of_week: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            milliseconds: u16,
        }
        unsafe extern "system" {
            fn GetLocalTime(lp_system_time: *mut WinSystemTime);
        }
        let mut st = WinSystemTime {
            year: 0,
            month: 0,
            day_of_week: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            milliseconds: 0,
        };
        unsafe { GetLocalTime(&mut st) };
        return LocalDateTime {
            year: i32::from(st.year),
            month: u32::from(st.month),
            day: u32::from(st.day),
            hour: u32::from(st.hour),
            minute: u32::from(st.minute),
            second: u32::from(st.second),
            unix,
        };
    }
    #[cfg(unix)]
    {
        #[repr(C)]
        struct Tm {
            tm_sec: i32,
            tm_min: i32,
            tm_hour: i32,
            tm_mday: i32,
            tm_mon: i32,
            tm_year: i32,
            tm_wday: i32,
            tm_yday: i32,
            tm_isdst: i32,
            _pad: [usize; 8],
        }
        unsafe extern "C" {
            fn localtime_r(timep: *const isize, result: *mut Tm) -> *mut Tm;
        }
        let t = unix as isize;
        let mut tm = Tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
            _pad: [0; 8],
        };
        let ptr = unsafe { localtime_r(&t, &mut tm) };
        if !ptr.is_null() {
            return LocalDateTime {
                year: tm.tm_year + 1900,
                month: (tm.tm_mon + 1).max(1) as u32,
                day: tm.tm_mday.max(1) as u32,
                hour: tm.tm_hour.max(0) as u32,
                minute: tm.tm_min.max(0) as u32,
                second: tm.tm_sec.max(0) as u32,
                unix,
            };
        }
    }
    #[allow(unreachable_code)]
    utc_from_unix(unix)
}

#[allow(dead_code)]
fn utc_from_unix(unix: u64) -> LocalDateTime {
    let days = (unix / 86_400) as i64;
    let secs = (unix % 86_400) as u32;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = (y + i64::from(month <= 2)) as i32;
    LocalDateTime {
        year,
        month,
        day,
        hour: secs / 3600,
        minute: (secs % 3600) / 60,
        second: secs % 60,
        unix,
    }
}

fn sanitize_source_name(name: &str) -> String {
    let safe: String = name
        .chars()
        .take(64)
        .map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = safe.trim_matches('-');
    if trimmed.is_empty() {
        "inspelning".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn format_recording_stem(template: &str, name: &str, now: LocalDateTime) -> String {
    let template = if template.trim().is_empty() {
        crate::settings::DEFAULT_FILENAME_FORMAT
    } else {
        template.trim()
    };
    let safe_name = sanitize_source_name(name);
    let date = format!("{:04}-{:02}-{:02}", now.year, now.month, now.day);
    let time = format!("{:02}-{:02}-{:02}", now.hour, now.minute, now.second);
    let rendered = template
        .replace("{namn}", &safe_name)
        .replace("{datum}", &date)
        .replace("{tid}", &time)
        .replace("{år}", &format!("{:04}", now.year))
        .replace("{månad}", &format!("{:02}", now.month))
        .replace("{dag}", &format!("{:02}", now.day))
        .replace("{timme}", &format!("{:02}", now.hour))
        .replace("{minut}", &format!("{:02}", now.minute))
        .replace("{sekund}", &format!("{:02}", now.second))
        .replace("{unix}", &now.unix.to_string());
    let sanitized: String = rendered
        .chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*')
            {
                '-'
            } else {
                ch
            }
        })
        .take(128)
        .collect();
    let trimmed = sanitized.trim_matches(|ch: char| ch.is_whitespace() || ch == '.');
    if trimmed.is_empty() {
        "inspelning".to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn preview_recording_filename(template: &str, name: &str, format: &str) -> String {
    let extension = match format {
        "WAV" => "wav",
        "FLAC" => "flac",
        "Opus" => "opus",
        "MP3" => "mp3",
        _ => "flac",
    };
    let stem = format_recording_stem(template, name, local_now());
    format!("{stem}.{extension}")
}

fn reserve_recording_path(
    folder: &Path,
    filename_format: &str,
    name: &str,
    extension: &str,
) -> io::Result<PathBuf> {
    let folder = if folder.as_os_str().is_empty() {
        Path::new(".")
    } else {
        folder
    };
    fs::create_dir_all(folder)?;
    let folder = std::path::absolute(folder)?;
    let stem = format_recording_stem(filename_format, name, local_now());
    for suffix in 0u32.. {
        let filename = if suffix == 0 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem}-{}.{extension}", suffix + 1)
        };
        let path = folder.join(filename);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    unreachable!()
}

fn encode(
    command: &mut Command,
    receiver: Receiver<Vec<f32>>,
    state: &State,
    active_status: &str,
    network: bool,
) -> (Receiver<Vec<f32>>, Result<(), String>) {
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return (receiver, Err(format!("Kunde inte starta FFmpeg: {error}"))),
    };
    let mut stdin = child.stdin.take().expect("FFmpeg stdin is piped");
    let stdout = child.stdout.take().expect("FFmpeg stdout is piped");
    let progress = AtomicI64::new(0);
    let cancel = AtomicBool::new(false);
    let writing = AtomicBool::new(false);

    thread::scope(|scope| {
        let progress_reader = scope.spawn(|| {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(time) = line
                    .strip_prefix("out_time_us=")
                    .and_then(|time| time.parse::<i64>().ok())
                    .filter(|time| *time > 0)
                {
                    progress.fetch_max(time, Relaxed);
                }
            }
        });
        let writer = {
            let cancel = &cancel;
            let writing = &writing;
            scope.spawn(move || {
                let mut bytes = Vec::new();
                while !cancel.load(Relaxed) {
                    let pcm = match receiver.recv_timeout(POLL) {
                        Ok(pcm) => pcm,
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => break,
                    };
                    bytes.clear();
                    bytes.extend(pcm.iter().flat_map(|sample| sample.to_le_bytes()));
                    writing.store(true, Relaxed);
                    let result = stdin.write_all(&bytes);
                    writing.store(false, Relaxed);
                    if result.is_err() {
                        break;
                    }
                }
                drop(stdin);
                receiver
            })
        };

        let result = monitor(
            &mut child,
            state,
            active_status,
            network,
            &progress,
            &writing,
        );
        cancel.store(true, Relaxed);
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = progress_reader.join();
        (writer.join().expect("FFmpeg writer panicked"), result)
    })
}

fn monitor(
    child: &mut std::process::Child,
    state: &State,
    active_status: &str,
    network: bool,
    progress: &AtomicI64,
    writing: &AtomicBool,
) -> Result<(), String> {
    let mut last_progress = 0;
    let mut stalled_since = None;
    let mut stopping_since = None;
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("FFmpeg: {error}"))?
        {
            if status.success() {
                return Ok(());
            }
            return Err(format!("FFmpeg avslutades med status {status}"));
        }
        if state.stop.load(Relaxed) {
            if network {
                return Ok(());
            }
            if stopping_since.is_none() {
                state.set_status("Slutför inspelningen…");
            }
            if stopping_since.get_or_insert_with(Instant::now).elapsed() > FINALIZE_TIMEOUT {
                return Err("Tidsgränsen för att slutföra inspelningen överskreds".into());
            }
        }
        let current_progress = progress.load(Relaxed);
        if current_progress != last_progress {
            if last_progress == 0 && stopping_since.is_none() {
                state.set_status(active_status);
            }
            state.encoded_us.store(current_progress, Relaxed);
            last_progress = current_progress;
            stalled_since = None;
        }
        if writing.load(Relaxed) {
            if stalled_since.get_or_insert_with(Instant::now).elapsed() > STALL_TIMEOUT {
                return Err("FFmpeg tar inte längre emot ljud".into());
            }
        } else {
            stalled_since = None;
        }
        thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests;
