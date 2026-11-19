use super::*;
use std::{
    f32::consts::TAU,
    io::Read,
    net::{Shutdown, TcpListener},
};

fn tone(amplitude: f32) -> Vec<f32> {
    (0..960)
        .map(|sample| amplitude * (TAU * sample as f32 / 48.0).sin())
        .collect()
}

fn stream_receiver(
    mount: &'static str,
    content_type: &'static str,
    magic: &'static [u8],
    attempts: usize,
) -> (Destination, Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (received, updates) = mpsc::channel();
    thread::spawn(move || {
        for _ in 0..attempts {
            let (socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut stream = BufReader::new(socket);
            let mut request = String::new();
            stream.read_line(&mut request).unwrap();
            assert_eq!(request.trim_end(), format!("PUT {mount} HTTP/1.1"));
            let mut headers = String::new();
            loop {
                let mut line = String::new();
                assert!(stream.read_line(&mut line).unwrap() > 0);
                if line == "\r\n" {
                    break;
                }
                headers.push_str(&line);
            }
            let header = |key: &str| {
                headers.lines().find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case(key).then_some(value.trim())
                })
            };
            assert_eq!(header("content-type"), Some(content_type));
            // Base64 for test-source:test-secret.
            assert_eq!(
                header("authorization"),
                Some("Basic dGVzdC1zb3VyY2U6dGVzdC1zZWNyZXQ=")
            );
            stream
                .get_mut()
                .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
                .unwrap();
            let mut first_bytes = vec![0; magic.len()];
            stream.read_exact(&mut first_bytes).unwrap();
            assert_eq!(first_bytes, magic);
            stream.get_ref().shutdown(Shutdown::Both).unwrap();
            received.send(()).unwrap();
        }
    });
    (
        Destination {
            id: 1,
            name: "Test".into(),
            server: format!("http://{address}"),
            mount: mount.into(),
            user: "test-source".into(),
            password: "test-secret".into(),
            codec: if content_type == "audio/mpeg" {
                "MP3"
            } else {
                "Opus"
            }
            .into(),
            bitrate: 96,
        },
        updates,
    )
}

#[test]
#[ignore = "requires FFmpeg with libopus/libmp3lame and local TCP access"]
fn records_four_formats_and_reconnects_without_interrupting_other_outputs() {
    let (opus_dest, opus_updates) = stream_receiver("/first.opus", "application/ogg", b"OggS", 2);
    let (mp3_dest, mp3_updates) = stream_receiver("/second.mp3", "audio/mpeg", b"ID3", 1);
    let streams = [&opus_dest, &mp3_dest].map(|dest| {
        let (target, tls) = StreamTarget::parse(dest).unwrap();
        Output::stream(dest, 1, &target, tls).unwrap()
    });
    let folder = tempfile::tempdir().unwrap();
    let formats = [("WAV", 24), ("FLAC", 24), ("Opus", 96), ("MP3", 128)];
    let mut recordings: Vec<_> = formats
        .iter()
        .map(|(format, quality)| {
            let settings = Recording {
                folder: folder.path().display().to_string(),
                format: (*format).into(),
                quality: *quality,
                ..Recording::default()
            };
            Output::record(&settings, "Källa", 1)
        })
        .collect();
    let samples: Vec<_> = (1..=4).map(|index| tone(index as f32 * 0.12)).collect();
    let deadline = Instant::now() + Duration::from_secs(18);
    let mut opus_connections = 0;
    let mut mp3_connections = 0;
    while Instant::now() < deadline {
        for stream in &streams {
            stream.push(&samples[0]);
        }
        for (recording, block) in recordings.iter().zip(&samples) {
            recording.push(block);
        }
        opus_connections += opus_updates.try_iter().count();
        mp3_connections += mp3_updates.try_iter().count();
        if opus_connections == 2 && mp3_connections == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let stream_status: Vec<_> = streams.iter().map(Output::status).collect();
    drop(streams);
    for recording in &mut recordings {
        recording.stop();
    }
    wait_for_shutdown();
    assert_eq!(opus_connections, 2, "{stream_status:?}");
    assert_eq!(mp3_connections, 1, "{stream_status:?}");

    let paths: Vec<_> = fs::read_dir(folder.path())
        .unwrap()
        .map(|item| item.unwrap().path())
        .collect();
    assert_eq!(paths.len(), formats.len());
    for (recording, (format, _)) in recordings.iter().zip(formats) {
        assert!(recording.finished());
        assert!(!recording.encoded_time().is_zero());
        let path = recording.recording_result().unwrap();
        assert!(paths.contains(&path));
        assert_eq!(path.extension().unwrap(), format.to_ascii_lowercase().as_str());
    }
    for (index, (format, _)) in formats.iter().enumerate() {
        let extension = format.to_ascii_lowercase();
        let path = paths
            .iter()
            .find(|path| path.extension().unwrap() == extension.as_str())
            .unwrap();
        let decoded = Command::new(ffmpeg_path())
            .args(["-v", "error", "-i"])
            .arg(path)
            .args(["-f", "f32le", "-acodec", "pcm_f32le", "pipe:1"])
            .output()
            .unwrap();
        assert!(
            decoded.status.success(),
            "{}",
            String::from_utf8_lossy(&decoded.stderr)
        );
        assert!(decoded.stdout.len() >= 48_000 * 4, "{format} is too short");
        let samples = decoded
            .stdout
            .chunks_exact(4)
            .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()));
        let count = samples.len();
        let rms = (samples.map(|sample| sample * sample).sum::<f32>() / count as f32).sqrt();
        let expected = (index + 1) as f32 * 0.12 / 2_f32.sqrt();
        assert!(
            (rms - expected).abs() < 0.03,
            "{format}: {rms} vs {expected}"
        );
    }
}

#[test]
fn formats_recording_filename_with_placeholders_and_sanitization() {
    let dt = LocalDateTime {
        year: 2026,
        month: 11,
        day: 5,
        hour: 9,
        minute: 4,
        second: 7,
        unix: 1_793_869_447,
    };
    assert_eq!(
        format_recording_stem("{namn}-{datum}-{tid}", "Sal A / Möte", dt),
        "Sal-A---Möte-2026-11-05-09-04-07"
    );
    assert_eq!(
        format_recording_stem(
            "{år}_{månad}_{dag}_{timme}{minut}{sekund}_{namn}",
            "Källa 1",
            dt
        ),
        "2026_11_05_090407_Källa-1"
    );
    assert_eq!(
        format_recording_stem("{namn}-{unix}", "Källa 1", dt),
        "Källa-1-1793869447"
    );
    assert_eq!(
        format_recording_stem("   ", "Källa 1", dt),
        "Källa-1-2026-11-05-09-04-07"
    );
    assert_eq!(
        format_recording_stem("../farlig:fil*namn?", "Källa", dt),
        "-farlig-fil-namn-"
    );
    assert_eq!(format_recording_stem("...", "Källa", dt), "inspelning");
}

#[test]
fn reserve_recording_path_avoids_overwriting_existing_files() {
    let folder = tempfile::tempdir().unwrap();
    let first = reserve_recording_path(folder.path(), "mote-{namn}", "Sal 1", "flac").unwrap();
    let second = reserve_recording_path(folder.path(), "mote-{namn}", "Sal 1", "flac").unwrap();
    let third = reserve_recording_path(folder.path(), "mote-{namn}", "Sal 1", "flac").unwrap();
    assert_eq!(first.file_name().unwrap(), "mote-Sal-1.flac");
    assert_eq!(second.file_name().unwrap(), "mote-Sal-1-2.flac");
    assert_eq!(third.file_name().unwrap(), "mote-Sal-1-3.flac");
}
