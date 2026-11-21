use std::{
    fs::{self, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
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

use base64::{Engine as _, engine::general_purpose::STANDARD};
use curl::easy::{Easy, HttpVersion, List, ReadError};
use url::Url;

use crate::{Destination, Recording, engine::BLOCK_DURATION};

const POLL: Duration = Duration::from_millis(50);
const RETRY_DELAY: Duration = Duration::from_secs(3);
const STALL_TIMEOUT: Duration = Duration::from_secs(12);
const FINALIZE_TIMEOUT: Duration = Duration::from_secs(8);
// FFmpeg can stop reading stdin while it waits for the Icecast handshake.
// Keep the mixer's audio in order for as long as we allow a blocked write.
const STREAM_QUEUE_BLOCKS: usize = STALL_TIMEOUT
    .as_millis()
    .div_ceil(BLOCK_DURATION.as_millis()) as usize;

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
    status: Mutex<OutputStatus>,
    stop: AtomicBool,
    finished: AtomicBool,
    dropped: AtomicUsize,
    encoded_us: AtomicI64,
    saved_file: Mutex<Option<PathBuf>>,
}

#[derive(Clone, Debug)]
pub enum OutputStatus {
    Progress(String),
    Problem(String),
}

impl State {
    fn set_status(&self, text: impl Into<String>) {
        *self.status.lock().unwrap() = OutputStatus::Progress(text.into());
    }

    fn set_problem(&self, text: impl Into<String>) {
        *self.status.lock().unwrap() = OutputStatus::Problem(text.into());
    }
}

impl Output {
    pub fn stream(
        dest: &Destination,
        channels: u16,
        target: &StreamTarget,
        tls: bool,
    ) -> Result<Self, &'static str> {
        let (command, upload) = stream_command(dest, channels, target, tls)?;
        Ok(Self::start(STREAM_QUEUE_BLOCKS, move |receiver, state| {
            run_stream(command, upload, channels, receiver, &state);
        }))
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
            status: Mutex::new(OutputStatus::Progress("Startar…".into())),
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
        if worker.is_err() {
            state.set_problem("Ljudbearbetningen kunde inte starta. Försök igen.");
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

    pub fn status(&self) -> OutputStatus {
        self.state.status.lock().unwrap().clone()
    }

    pub fn dropped_audio(&self) -> bool {
        self.state.dropped.load(Relaxed) > 0
    }

    pub fn encoded_time(&self) -> Duration {
        Duration::from_micros(self.state.encoded_us.load(Relaxed) as u64)
    }

    pub fn stop(&mut self) {
        self.state.stop.store(true, Relaxed);
        self.sender.take();
    }

    pub fn recording_result(&self) -> Result<PathBuf, String> {
        self.state
            .saved_file
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| match self.status() {
                OutputStatus::Problem(reason) => reason,
                OutputStatus::Progress(_) => {
                    "Ingen sparad fil kunde bekräftas. Kontrollera inspelningsmappen.".into()
                }
            })
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
            "quiet",
            "-nostdin",
            "-progress",
            "pipe:2",
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
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    command
}

#[derive(PartialEq, Eq)]
pub struct StreamTarget {
    host: String,
    port: u16,
    mount: String,
}

impl StreamTarget {
    pub fn mount(&self) -> &str {
        &self.mount
    }

    pub fn server_url(&self, tls: bool) -> String {
        let scheme = if tls { "https" } else { "http" };
        format!("{scheme}://{}:{}", self.host, self.port)
    }

    pub fn parse(dest: &Destination) -> Result<(Self, bool), &'static str> {
        let raw = dest.server.trim();
        if raw.chars().any(char::is_control) {
            return Err("Ta bort ogiltiga kontrolltecken från serveradressen.");
        }
        let Some((scheme, rest)) = raw.split_once("://") else {
            return Err(
                "Kontrollera serveradressen under Destinationer. Ange en adress som börjar med http:// eller https://.",
            );
        };
        let tls = if scheme.eq_ignore_ascii_case("http") {
            false
        } else if scheme.eq_ignore_ascii_case("https") {
            true
        } else {
            return Err("Serveradressen måste börja med http:// eller https://.");
        };
        if rest.is_empty() || rest.starts_with(['/', '\\', '?', '#']) {
            return Err("Lägg till serverns värdnamn i serveradressen under Destinationer.");
        }
        if rest
            .split(['/', '\\', '?', '#'])
            .next()
            .is_some_and(|authority| authority.ends_with(':'))
        {
            return Err("Ange en serverport mellan 1 och 65535 under Destinationer.");
        }
        let server = Url::parse(raw)
            .map_err(|_| "Kontrollera serveradressen under Destinationer. Ange en adress som börjar med http:// eller https://.")?;
        let port = server
            .port_or_known_default()
            .filter(|port| *port > 0)
            .ok_or("Ange en serverport mellan 1 och 65535 under Destinationer.")?;
        let host = server
            .host()
            .ok_or("Lägg till serverns värdnamn i serveradressen under Destinationer.")?
            .to_string();
        let mount = normalize_mount(&dest.mount).ok_or(
            "Kontrollera monteringspunkten under Destinationer. Ange till exempel /live.",
        )?;
        Ok((Self { host, port, mount }, tls))
    }
}

pub(crate) fn normalize_mount(value: &str) -> Option<String> {
    let mount = value.trim().trim_start_matches('/');
    if mount.is_empty() || mount.contains(['?', '#']) || mount.chars().any(char::is_control) {
        return None;
    }
    let mut url = Url::parse("http://localhost").ok()?;
    url.set_path(&format!("/{mount}"));
    let normalized = url.path().to_owned();
    if normalized.trim_start_matches('/').is_empty() {
        return None;
    }
    Some(normalized)
}

pub fn bitrate_range(format: &str) -> Option<std::ops::RangeInclusive<u32>> {
    if format.eq_ignore_ascii_case("mp3") {
        Some(32..=320)
    } else if format.eq_ignore_ascii_case("opus") {
        Some(16..=510)
    } else {
        None
    }
}

pub fn username_hint(value: &str) -> Option<&'static str> {
    let user = value.trim();
    (user.contains([':', '/', '?', '#', '@', '%'])
        || user.chars().any(|c| c.is_whitespace() || c.is_control()))
    .then_some(
        "Ta bort blanksteg, kontrolltecken och tecknen : / ? # @ % från Icecast-användarnamnet.",
    )
}

fn stream_command(
    dest: &Destination,
    channels: u16,
    target: &StreamTarget,
    tls: bool,
) -> Result<(Command, StreamUpload), &'static str> {
    if channels == 0 {
        return Err("Välj minst en ljudkanal.");
    }
    let user = dest.user.trim();
    if let Some(reason) = username_hint(user) {
        return Err(reason);
    }
    let user = if user.is_empty() { "source" } else { user };
    let (encoder, muxer, content_type) = match dest.codec.as_str() {
        "Opus" => ("libopus", "ogg", "application/ogg"),
        "MP3" => ("libmp3lame", "mp3", "audio/mpeg"),
        _ => return Err("Välj Opus eller MP3 under Kodek i destinationens inställningar."),
    };
    if let Some(range) = bitrate_range(&dest.codec)
        && !range.contains(&dest.bitrate)
    {
        return Err(if dest.codec == "MP3" {
            "Välj en bithastighet mellan 32 och 320 kbit/s för MP3."
        } else {
            "Välj en bithastighet mellan 16 och 510 kbit/s för Opus."
        });
    }
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
        "pipe:1",
    ]);
    Ok((
        command,
        StreamUpload {
            url: format!("{}{}", target.server_url(tls), target.mount()),
            authorization: format!(
                "Authorization: Basic {}",
                STANDARD.encode(format!("{user}:{}", dest.password))
            ),
            content_type,
        },
    ))
}

struct StreamUpload {
    url: String,
    authorization: String,
    content_type: &'static str,
}

impl StreamUpload {
    fn send(
        &self,
        mut audio: impl Read,
        state: &State,
        progress: &EncodeProgress,
    ) -> Result<(), String> {
        let mut http = Easy::new();
        let result = (|| -> Result<(), curl::Error> {
            http.url(&self.url)?;
            http.follow_location(false)?;
            http.ssl_verify_peer(true)?;
            http.ssl_verify_host(true)?;
            http.http_version(HttpVersion::V11)?;
            http.upload(true)?;
            http.fail_on_error(true)?;
            http.connect_timeout(Duration::from_secs(8))?;
            http.low_speed_limit(1)?;
            http.low_speed_time(STALL_TIMEOUT)?;
            http.progress(true)?;
            let mut headers = List::new();
            // Scope credentials to the origin request. A proxy challenge during
            // CONNECT must have no origin credentials available for negotiation.
            http.proxy_headers(List::new())?;
            headers.append(&self.authorization)?;
            headers.append(&format!("Content-Type: {}", self.content_type))?;
            // Icecast 2.4 expects raw audio after the PUT handshake.
            headers.append("Transfer-Encoding:")?;
            headers.append("Expect: 100-continue")?;
            headers.append("Ice-Public: 0")?;
            http.http_headers(headers)?;
            let mut transfer = http.transfer();
            transfer.read_function(|buf| audio.read(buf).map_err(|_| ReadError::Abort))?;
            transfer.write_function(|body| Ok(body.len()))?;
            transfer.progress_function(|_, _, _, sent| {
                if sent > 0.0 {
                    progress.sending.store(true, Relaxed);
                }
                !state.stop.load(Relaxed) && !progress.cancel.load(Relaxed)
            })?;
            transfer.perform()
        })();
        if state.stop.load(Relaxed) {
            return Ok(());
        }
        if result
            .as_ref()
            .is_err_and(|err| err.is_peer_failed_verification())
        {
            return Err("Serverns TLS-certifikat kunde inte verifieras. Kontrollera serveradress och certifikat.".into());
        }
        match http.response_code().unwrap_or_default() {
            401 | 403 => {
                Err("Icecast nekade inloggningen. Kontrollera användarnamn och lösenord.".into())
            }
            300..=399 => Err(
                "Icecast omdirigerade sändningen. Ange destinationens slutliga serveradress."
                    .into(),
            ),
            _ => Err(
                "Anslutningen till Icecast bröts. Kontrollera nätverket och destinationen.".into(),
            ),
        }
    }
}

fn run_stream(
    mut command: Command,
    upload: StreamUpload,
    channels: u16,
    mut receiver: Receiver<Vec<f32>>,
    state: &State,
) {
    state.set_status("Ansluter…");
    while !state.stop.load(Relaxed) {
        let (returned, result) = encode(
            &mut command,
            receiver,
            state,
            "Sänder",
            Some(&upload),
            channels,
        );
        receiver = returned;
        if state.stop.load(Relaxed) {
            return;
        }
        state.encoded_us.store(0, Relaxed);
        let reason = result
            .err()
            .unwrap_or_else(|| "Anslutningen till servern bröts.".into());
        state.set_problem(format!("{reason} Appen försöker igen automatiskt."));
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
    if channels == 0 {
        state.set_problem("Välj minst en ljudkanal.");
        return;
    }
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
            state.set_problem("Välj WAV, FLAC, Opus eller MP3 under Lokal inspelning.");
            return;
        }
    };
    if let Some(range) = bitrate_range(format)
        && !range.contains(&quality)
    {
        state.set_problem(format!(
            "Välj en bithastighet mellan {} och {} kbit/s för {format}.",
            range.start(),
            range.end()
        ));
        return;
    }
    let (path, file) = match reserve_recording_path(
        Path::new(&settings.folder),
        &settings.filename_format,
        name,
        extension,
    ) {
        Ok(path) => path,
        Err(error) => {
            let hint = crate::feedback::file_hint(&error);
            state.set_problem(format!(
                "Kunde inte skapa inspelningsfilen i ”{}”. {hint}",
                settings.folder
            ));
            return;
        }
    };
    let identity = match file.try_clone().and_then(same_file::Handle::from_file) {
        Ok(handle) => handle,
        Err(err) => {
            state.set_problem(format!(
                "Kunde inte kontrollera inspelningsfilen. {}",
                crate::feedback::file_hint(&err)
            ));
            return;
        }
    };
    let mut command = ffmpeg(channels);
    command.args(["-c:a", encoder]);
    if format == "FLAC" {
        command.args(["-sample_fmt", if quality >= 24 { "s32" } else { "s16" }]);
    }
    if matches!(format, "Opus" | "MP3") {
        command.args(["-b:a", &format!("{quality}k")]);
    }
    if format == "WAV" {
        command.args(["-rf64", "auto"]);
    }
    // The encoder inherits the exclusively created, seekable file. A later rename
    // or replacement of the directory entry cannot redirect its writes.
    command.args(["-f", muxer, "fd:"]).stdout(file);
    let filename = path.file_name().unwrap().to_string_lossy();
    let (_, result) = encode(
        &mut command,
        receiver,
        state,
        &format!("Spelar in · {filename}"),
        None,
        channels,
    );
    match result.and_then(|_| verify_recording_path(&path, &identity)) {
        Ok(()) => {
            state.set_status(format!("Sparad · {filename}"));
            *state.saved_file.lock().unwrap() = Some(path);
        }
        Err(error) => {
            state.set_problem(format!("{error} Filnamn: {filename}"));
        }
    }
}

fn verify_recording_path(path: &Path, identity: &same_file::Handle) -> Result<(), String> {
    if same_file::Handle::from_path(path).is_ok_and(|current| &current == identity) {
        Ok(())
    } else {
        Err("Inspelningsfilen har flyttats, ersatts eller blivit otillgänglig. Kontrollera inspelningsmappen.".into())
    }
}

#[derive(Clone, Copy)]
struct LocalDateTime {
    year: i64,
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
            year: i64::from(st.year),
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
        let t = libc::time_t::try_from(unix).ok();
        let mut tm = std::mem::MaybeUninit::<libc::tm>::uninit();
        if let Some(t) = t
            // localtime_r initializes the platform's struct tm on success.
            && !unsafe { libc::localtime_r(&t, tm.as_mut_ptr()) }.is_null()
        {
            let tm = unsafe { tm.assume_init() };
            return LocalDateTime {
                year: i64::from(tm.tm_year) + 1900,
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
    let year = y + i64::from(month <= 2);
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

pub(crate) fn is_windows_reserved_name(stem: &str) -> bool {
    let base = stem.split('.').next().unwrap_or("").trim_end();
    matches!(
        base.to_ascii_uppercase().as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM0"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT0"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
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
    let time = format!("{:02}-{:02}", now.hour, now.minute);
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
    if trimmed.is_empty() || is_windows_reserved_name(trimmed) {
        "inspelning".to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub fn local_hm() -> String {
    let now = local_now();
    format!("{:02}:{:02}", now.hour, now.minute)
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
) -> io::Result<(PathBuf, fs::File)> {
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
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
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
    upload: Option<&StreamUpload>,
    channels: u16,
) -> (Receiver<Vec<f32>>, Result<(), String>) {
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let message = match error.kind() {
                io::ErrorKind::NotFound => {
                    "Ljudprogrammet FFmpeg saknas. Installera FFmpeg eller installera om Mötessändare."
                }
                io::ErrorKind::PermissionDenied => {
                    "Åtkomst till ljudprogrammet FFmpeg nekades. Kontrollera att du får köra programmet."
                }
                _ => "Ljudprogrammet FFmpeg kunde inte starta. Kontrollera installationen.",
            };
            return (receiver, Err(message.into()));
        }
    };
    let mut stdin = child.stdin.take().expect("FFmpeg stdin is piped");
    let stderr = child.stderr.take().expect("FFmpeg progress pipe");
    let progress = EncodeProgress::default();

    thread::scope(|scope| {
        let sender = upload.map(|upload| {
            let audio = child.stdout.take().expect("FFmpeg audio pipe");
            let progress = &progress;
            scope.spawn(move || upload.send(audio, state, progress))
        });
        let progress_reader = scope.spawn(|| {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if let Some(time) = line
                    .strip_prefix("out_time_us=")
                    .and_then(|time| time.parse::<i64>().ok())
                    .filter(|time| *time > 0)
                {
                    progress.encoded.fetch_max(time, Relaxed);
                }
            }
        });
        let writer = {
            let cancel = &progress.cancel;
            let writing = &progress.writing;
            scope.spawn(move || {
                let mut bytes = Vec::new();
                let mut samples_written = 0u64;
                let mut drained = false;
                while !cancel.load(Relaxed) {
                    let pcm = match receiver.recv_timeout(POLL) {
                        Ok(pcm) => pcm,
                        Err(RecvTimeoutError::Timeout) => continue,
                        Err(RecvTimeoutError::Disconnected) => {
                            drained = true;
                            break;
                        }
                    };
                    bytes.clear();
                    bytes.extend(pcm.iter().flat_map(|sample| sample.to_le_bytes()));
                    writing.store(true, Relaxed);
                    let result = stdin.write_all(&bytes);
                    writing.store(false, Relaxed);
                    if result.is_err() {
                        break;
                    }
                    samples_written += pcm.len() as u64;
                }
                drop(stdin);
                (receiver, samples_written, drained)
            })
        };

        let result = monitor(&mut child, state, active_status, sender.as_ref(), &progress);
        progress.cancel.store(true, Relaxed);
        if child.try_wait().ok().flatten().is_none() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = progress_reader.join();
        let (receiver, samples_written, drained) = writer.join().expect("FFmpeg writer panicked");
        let result = if let Some(sender) = sender {
            let sent = sender.join().expect("Icecast uploader panicked");
            result.and(sent)
        } else if result.is_ok() && !drained {
            Err("FFmpeg avslutades innan allt inspelningsljud hade skrivits.".into())
        } else {
            // Some FFmpeg versions report the start timestamp of the last packet.
            // Once the trailer is written we know the exact duration from the PCM.
            if result.is_ok() {
                let frames = samples_written / u64::from(channels);
                state
                    .encoded_us
                    .store((u128::from(frames) * 1_000_000 / 48_000) as i64, Relaxed);
            }
            result
        };
        (receiver, result)
    })
}

#[derive(Default)]
struct EncodeProgress {
    encoded: AtomicI64,
    sending: AtomicBool,
    writing: AtomicBool,
    cancel: AtomicBool,
}

fn monitor(
    child: &mut std::process::Child,
    state: &State,
    active_status: &str,
    sender: Option<&thread::ScopedJoinHandle<'_, Result<(), String>>>,
    progress: &EncodeProgress,
) -> Result<(), String> {
    let mut last_progress = 0;
    let mut stalled_since = None;
    let mut stopping_since = None;
    loop {
        if sender.is_some_and(|sender| sender.is_finished()) {
            return Ok(());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|_| "Kontakten med ljudprogrammet FFmpeg bröts.")?
        {
            if status.success() {
                return Ok(());
            }
            return Err(if sender.is_some() {
                "Ljudet kunde inte skickas till Icecast. Kontrollera nätverket, serveradressen och inloggningsuppgifterna."
            } else {
                "Ljudet kunde inte skrivas till filen. Kontrollera ledigt utrymme och att du får skriva i inspelningsmappen."
            }.into());
        }
        if state.stop.load(Relaxed) {
            if sender.is_some() {
                return Ok(());
            }
            if stopping_since.is_none() {
                state.set_status("Slutför inspelningen…");
            }
            if stopping_since.get_or_insert_with(Instant::now).elapsed() > FINALIZE_TIMEOUT {
                return Err("Det tog för lång tid att spara klart. Kontrollera om filen i inspelningsmappen går att spela upp.".into());
            }
        }
        let current_progress = if sender.is_none() || progress.sending.load(Relaxed) {
            progress.encoded.load(Relaxed)
        } else {
            0
        };
        if current_progress != last_progress {
            if last_progress == 0 && stopping_since.is_none() {
                state.set_status(active_status);
            }
            state.encoded_us.store(current_progress, Relaxed);
            last_progress = current_progress;
            stalled_since = None;
        }
        if progress.writing.load(Relaxed) {
            if stalled_since.get_or_insert_with(Instant::now).elapsed() > STALL_TIMEOUT {
                return Err("Ljudprogrammet FFmpeg har slutat ta emot ljud.".into());
            }
        } else {
            stalled_since = None;
        }
        thread::sleep(POLL);
    }
}

#[cfg(test)]
mod tests;
