use crate::{Destination, output::StreamTarget};
use eframe::egui::Context;
use serde::Deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::{Duration, Instant};
use url::Url;

const REFRESH_INTERVAL: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;
const MAX_HISTORY_SAMPLES: usize = 24;

pub struct StatsTarget {
    pub server: String,
    pub mount: String,
}

impl StatsTarget {
    pub fn parse(destination: &Destination) -> Result<Self, &'static str> {
        let (target, tls) = StreamTarget::parse(destination)?;
        Ok(Self {
            server: target.server_url(tls),
            mount: target.mount().to_owned(),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ListenerCounts {
    pub current: Option<u64>,
    pub peak: Option<u64>,
}

type ServerResult = Result<HashMap<String, ListenerCounts>, String>;

#[derive(Default)]
pub struct ListenerStats {
    servers: HashMap<String, ServerStats>,
    history: HashMap<(String, String), VecDeque<u64>>,
}

#[derive(Default)]
struct ServerStats {
    pending: Option<Receiver<ServerResult>>,
    result: Option<ServerResult>,
    next_refresh: Option<Instant>,
}

pub enum MountStatus<'a> {
    Loading,
    Unavailable(&'a str),
    Absent,
    Listed(&'a ListenerCounts),
}

impl ListenerStats {
    pub fn update(&mut self, ctx: &Context, servers: HashSet<&str>, refresh: bool) {
        self.servers
            .retain(|server, _| servers.contains(server.as_str()));
        self.history
            .retain(|(server, _), _| servers.contains(server.as_str()));
        let now = Instant::now();
        for server in servers {
            let stats = self.servers.entry(server.to_owned()).or_default();
            if let Some(pending) = &stats.pending {
                let received = match pending.try_recv() {
                    Ok(result) => result,
                    Err(TryRecvError::Empty) => continue,
                    Err(TryRecvError::Disconnected) => {
                        Err("Hämtningen avbröts. Välj Uppdatera för att försöka igen.".into())
                    }
                };
                if let Ok(mounts) = &received {
                    for (mount, counts) in mounts {
                        if let Some(current) = counts.current {
                            let samples = self
                                .history
                                .entry((server.to_owned(), mount.clone()))
                                .or_default();
                            if samples.len() >= MAX_HISTORY_SAMPLES {
                                samples.pop_front();
                            }
                            samples.push_back(current);
                        }
                    }
                }
                stats.result = Some(received);
                stats.pending = None;
            }

            if refresh || stats.next_refresh.is_none_or(|next| now >= next) {
                stats.next_refresh = Some(now + REFRESH_INTERVAL);
                let (sender, receiver) = mpsc::channel();
                let url = format!("{server}/status-json.xsl");
                let ctx = ctx.clone();
                match std::thread::Builder::new()
                    .name("icecast-stats".into())
                    .spawn(move || {
                        let result = fetch(&url);
                        if sender.send(result).is_ok() {
                            ctx.request_repaint();
                        }
                    }) {
                    Ok(_) => stats.pending = Some(receiver),
                    Err(_) => {
                        stats.result = Some(Err(
                            "Statistiken kunde inte hämtas. Försök igen om en stund.".into(),
                        ))
                    }
                }
            }
        }
    }

    pub fn refreshing(&self) -> bool {
        self.servers.values().any(|stats| stats.pending.is_some())
    }

    pub fn history(&self, target: &StatsTarget) -> Option<&VecDeque<u64>> {
        self.history
            .get(&(target.server.clone(), target.mount.clone()))
            .filter(|samples| samples.len() >= 2)
    }

    pub fn status(&self, target: &StatsTarget) -> MountStatus<'_> {
        match self
            .servers
            .get(&target.server)
            .and_then(|stats| stats.result.as_ref())
        {
            Some(Ok(mounts)) => mounts
                .get(&target.mount)
                .map_or(MountStatus::Absent, MountStatus::Listed),
            Some(Err(error)) => MountStatus::Unavailable(error),
            None => MountStatus::Loading,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionProbeStatus {
    Testing,
    Ok(String),
    Failed(String),
}

#[derive(Default)]
pub struct ConnectionTester {
    pending: HashMap<u64, Receiver<Result<String, String>>>,
    results: HashMap<u64, Result<String, String>>,
}

impl ConnectionTester {
    pub fn poll(&mut self) {
        let finished: Vec<(u64, Result<String, String>)> = self
            .pending
            .iter()
            .filter_map(|(&id, rx)| match rx.try_recv() {
                Ok(res) => Some((id, res)),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some((id, Err("Testet avbröts".to_owned()))),
            })
            .collect();
        for (id, res) in finished {
            self.pending.remove(&id);
            self.results.insert(id, res);
        }
    }

    pub fn clear(&mut self, dest_id: u64) {
        self.pending.remove(&dest_id);
        self.results.remove(&dest_id);
    }

    pub fn start(&mut self, ctx: &Context, dest: &Destination) {
        let dest_id = dest.id;
        let target = match StatsTarget::parse(dest) {
            Ok(target) => target,
            Err(error) => {
                self.pending.remove(&dest_id);
                self.results.insert(dest_id, Err(error.to_owned()));
                return;
            }
        };
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        self.results.remove(&dest_id);
        match std::thread::Builder::new()
            .name("icecast-probe".into())
            .spawn(move || {
                let res = probe_server(&target.server);
                if tx.send(res).is_ok() {
                    ctx.request_repaint();
                }
            }) {
            Ok(_) => {
                self.pending.insert(dest_id, rx);
            }
            Err(_) => {
                self.results.insert(
                    dest_id,
                    Err("Kunde inte starta anslutningstestet".to_owned()),
                );
            }
        }
    }

    pub fn status(&self, dest_id: u64) -> Option<ConnectionProbeStatus> {
        if self.pending.contains_key(&dest_id) {
            return Some(ConnectionProbeStatus::Testing);
        }
        self.results.get(&dest_id).map(|res| match res {
            Ok(msg) => ConnectionProbeStatus::Ok(msg.clone()),
            Err(err) => ConnectionProbeStatus::Failed(err.clone()),
        })
    }
}

fn probe_server(server: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|_| "Kunde inte starta HTTP-klienten".to_owned())?;
    let started = Instant::now();
    let status_url = format!("{server}/status-json.xsl");
    let response = client.get(&status_url).send().map_err(|error| {
        if error.is_timeout() {
            "Servern svarade inte i tid".to_owned()
        } else {
            "Kunde inte ansluta till servern".to_owned()
        }
    })?;
    let latency_ms = started.elapsed().as_millis();
    let code = response.status();
    if code.is_success() {
        let mut body = Vec::new();
        let _ = response.take(MAX_RESPONSE_BYTES).read_to_end(&mut body);
        if let Ok(parsed) = serde_json::from_slice::<ProbeResponse>(&body)
            && let Some(server_id) = parsed.icestats.server_id.filter(|s| !s.trim().is_empty())
        {
            return Ok(format!("{server_id} nåddes ({latency_ms} ms)"));
        }
        return Ok(format!("Servern nåddes ({latency_ms} ms)"));
    }
    if code.as_u16() == 404 || code.as_u16() == 403 {
        return Ok(format!(
            "Servern svarade (HTTP {}, {latency_ms} ms)",
            code.as_u16()
        ));
    }
    Err(format!("Servern returnerade HTTP {}", code.as_u16()))
}

#[derive(Deserialize)]
struct ProbeResponse {
    icestats: ProbeIceStats,
}

#[derive(Deserialize)]
struct ProbeIceStats {
    server_id: Option<String>,
}

fn fetch(url: &str) -> ServerResult {
    let client = reqwest::blocking::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|_| "Appen kunde inte öppna en nätverksanslutning för att hämta statistiken.")?;
    let response = client.get(url).send().map_err(|error| {
        if error.is_timeout() {
            "Icecast-servern svarade inte i tid. Försök igen om en stund."
        } else {
            "Icecast-servern kunde inte nås. Kontrollera nätverket och serveradressen under Destinationer."
        }
    })?;
    read_response(response.status(), response)
}

fn read_response(status: reqwest::StatusCode, response: impl Read) -> ServerResult {
    if !status.is_success() {
        let code = status.as_u16();
        let message = match code {
            401 | 403 => "Servern tillåter inte att appen läser den publika lyssnarstatistiken.",
            404 => {
                "Statistiksidan saknas på servern. Kontrollera att adressen går till rätt Icecast-server."
            }
            _ => "Servern kunde inte lämna ut lyssnarstatistiken. Försök igen om en stund.",
        };
        return Err(format!("{message} (HTTP {code})"));
    }
    let mut body = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|_| "Hela statistiksvaret kunde inte läsas. Försök igen om en stund.")?;
    if body.len() as u64 > MAX_RESPONSE_BYTES {
        return Err("Serverns statistiksida är för stor för att visas.".into());
    }
    parse(&body)
}

#[derive(Deserialize)]
struct StatusResponse {
    icestats: IceStats,
}

#[derive(Deserialize)]
struct IceStats {
    source: Option<Sources>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Sources {
    Single(Stream),
    Multiple(Vec<Stream>),
}

#[derive(Deserialize)]
struct Stream {
    listenurl: Option<String>,
    listeners: Option<Count>,
    listener_peak: Option<Count>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Count {
    Number(u64),
    Text(String),
}

impl Count {
    fn value(self) -> Option<u64> {
        match self {
            Self::Number(count) => Some(count),
            Self::Text(text) => text.trim().parse().ok(),
        }
    }
}

fn parse(body: &[u8]) -> ServerResult {
    let response: StatusResponse = serde_json::from_slice(body)
        .map_err(|_| "Serverns svar gick inte att läsa som lyssnarstatistik. Kontrollera att adressen går till rätt Icecast-server.")?;
    let streams = match response.icestats.source {
        Some(Sources::Single(stream)) => vec![stream],
        Some(Sources::Multiple(streams)) => streams,
        None => Vec::new(),
    };
    Ok(streams
        .into_iter()
        .filter_map(|stream| {
            // Icecast can advertise a different hostname from the one used to query it.
            let url = Url::parse(stream.listenurl.as_deref()?).ok()?;
            let counts = ListenerCounts {
                current: stream.listeners.and_then(Count::value),
                peak: stream.listener_peak.and_then(Count::value),
            };
            Some((url.path().to_owned(), counts))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    #[test]
    fn target_matches_stream_mount_and_excludes_credentials() {
        let destination = Destination {
            server: "https://source:secret@[::1]:8443/path?token=secret".into(),
            mount: " /Möte live.opus ".into(),
            ..Destination::default()
        };
        let target = StatsTarget::parse(&destination).unwrap();
        assert_eq!(target.server, "https://[::1]:8443");
        let mounts = parse(
            br#"{"icestats":{"source":{
            "listenurl":"http://public.example/M%C3%B6te%20live.opus",
            "listeners":0,"listener_peak":"12"
        }}}"#,
        )
        .unwrap();
        assert_eq!(
            mounts[&target.mount],
            ListenerCounts {
                current: Some(0),
                peak: Some(12)
            }
        );
    }

    #[test]
    fn parses_multiple_mounts_and_preserves_missing_counts() {
        let mounts = parse(
            br#"{"icestats":{"source":[
            {"listenurl":"http://icecast.example/first", "listeners":"7"},
            {"listenurl":"http://icecast.example/second", "listener_peak":17}
        ]}}"#,
        )
        .unwrap();
        assert_eq!(
            mounts["/first"],
            ListenerCounts {
                current: Some(7),
                peak: None
            }
        );
        assert_eq!(
            mounts["/second"],
            ListenerCounts {
                current: None,
                peak: Some(17)
            }
        );
        assert!(
            parse(br#"{"icestats":{"server_id":"Icecast"}}"#)
                .unwrap()
                .is_empty()
        );
        assert!(parse(b"<html>Maintenance</html>").is_err());
    }

    #[test]
    fn refresh_keeps_last_counts_until_replaced_and_reports_fetch_errors() {
        let target = StatsTarget {
            server: "http://example.invalid".into(),
            mount: "/live".into(),
        };
        let mut stats = ListenerStats::default();
        assert!(matches!(stats.status(&target), MountStatus::Loading));
        let counts = ListenerCounts {
            current: Some(4),
            peak: Some(6),
        };
        let (sender, receiver) = mpsc::channel();
        stats.servers.insert(
            target.server.clone(),
            ServerStats {
                pending: Some(receiver),
                result: Some(Ok(HashMap::from([(target.mount.clone(), counts)]))),
                next_refresh: Some(Instant::now() + REFRESH_INTERVAL),
            },
        );
        let servers = HashSet::from([target.server.as_str()]);
        let ctx = Context::default();
        stats.update(&ctx, servers.clone(), true);
        assert!(stats.refreshing());
        assert!(
            matches!(stats.status(&target), MountStatus::Listed(counts) if counts.current == Some(4))
        );

        sender.send(Err("Serverfel".into())).unwrap();
        stats.update(&ctx, servers, false);
        assert!(!stats.refreshing());
        assert!(matches!(
            stats.status(&target),
            MountStatus::Unavailable("Serverfel")
        ));
        stats.update(&ctx, HashSet::new(), false);
        assert!(matches!(stats.status(&target), MountStatus::Loading));
    }

    #[test]
    fn distinguishes_access_denied_missing_status_page_and_server_failure() {
        use reqwest::StatusCode;

        for (status, hint) in [
            (StatusCode::UNAUTHORIZED, "tillåter inte"),
            (StatusCode::FORBIDDEN, "tillåter inte"),
            (StatusCode::NOT_FOUND, "Statistiksidan saknas"),
            (StatusCode::SERVICE_UNAVAILABLE, "Försök igen"),
        ] {
            let error = read_response(status, &b"<html>Error</html>"[..]).unwrap_err();
            assert!(error.contains(hint), "{error}");
            assert!(
                error.contains(&format!("HTTP {}", status.as_u16())),
                "{error}"
            );
        }
    }

    #[test]
    fn accepts_the_largest_status_document_and_limits_reads_from_oversized_ones() {
        let mut body = br#"{"icestats":{}}"#.to_vec();
        body.resize(MAX_RESPONSE_BYTES as usize, b' ');
        assert!(
            read_response(reqwest::StatusCode::OK, body.as_slice())
                .unwrap()
                .is_empty()
        );

        body.extend_from_slice(b"  ");
        let mut unread = body.as_slice();
        let error = read_response(reqwest::StatusCode::OK, &mut unread).unwrap_err();
        assert!(error.contains("för stor"), "{error}");
        assert_eq!(unread, b" ");
    }

    #[test]
    #[ignore = "requires local TCP access"]
    fn fetches_public_status_over_http() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/status-json.xsl", listener.local_addr().unwrap());
        let worker = std::thread::spawn(move || fetch(&url));

        let deadline = Instant::now() + REQUEST_TIMEOUT;
        let mut connection = loop {
            match listener.accept() {
                Ok((connection, _)) => break connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if worker.is_finished() || Instant::now() >= deadline {
                        panic!("Client did not connect: {:?}", worker.join().unwrap());
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("Could not accept connection: {error}"),
            }
        };
        connection.set_nonblocking(false).unwrap();
        connection.set_read_timeout(Some(REQUEST_TIMEOUT)).unwrap();
        let mut request = BufReader::new(&mut connection);
        let mut line = String::new();
        request.read_line(&mut line).unwrap();
        assert_eq!(line, "GET /status-json.xsl HTTP/1.1\r\n");
        loop {
            line.clear();
            assert_ne!(request.read_line(&mut line).unwrap(), 0);
            if line == "\r\n" {
                break;
            }
            assert!(!line.to_ascii_lowercase().starts_with("authorization:"));
        }
        let body = r#"{"icestats":{"source":{"listenurl":"http://localhost/live","listeners":3}}}"#;
        write!(
            connection,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        assert_eq!(worker.join().unwrap().unwrap()["/live"].current, Some(3));
    }
}
