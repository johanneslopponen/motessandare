use super::*;
use std::{
    f32::consts::TAU,
    io::Read,
    net::{TcpListener, TcpStream},
};

fn tone(amplitude: f32) -> Vec<f32> {
    (0..960)
        .map(|sample| amplitude * (TAU * sample as f32 / 48.0).sin())
        .collect()
}

fn decode_audio(path: &Path) -> Vec<f32> {
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
    decoded
        .stdout
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
        .collect()
}

fn stream_receiver(
    mount: &'static str,
    content_type: &'static str,
    magic: &'static [u8],
    attempts: usize,
    handshake_delay: Duration,
) -> (Destination, Receiver<BufReader<TcpStream>>) {
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
            thread::sleep(handshake_delay);
            stream
                .get_mut()
                .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")
                .unwrap();
            let mut first_bytes = vec![0; magic.len()];
            stream.read_exact(&mut first_bytes).unwrap();
            assert_eq!(first_bytes, magic);
            received.send(stream).unwrap();
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
    let (opus_dest, opus_updates) =
        stream_receiver("/first.opus", "application/ogg", b"OggS", 2, Duration::ZERO);
    let (mp3_dest, mp3_updates) =
        stream_receiver("/second.mp3", "audio/mpeg", b"ID3", 1, Duration::ZERO);
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
        let samples = decode_audio(path);
        assert!(samples.len() >= 48_000, "{format} is too short");
        let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt();
        let expected = (index + 1) as f32 * 0.12 / 2_f32.sqrt();
        assert!(
            (rms - expected).abs() < 0.03,
            "{format}: {rms} vs {expected}"
        );
    }
}

#[test]
#[ignore = "requires FFmpeg with libmp3lame and local TCP access"]
fn slow_icecast_handshake_preserves_startup_audio() {
    let (dest, connections) = stream_receiver(
        "/slow.mp3",
        "audio/mpeg",
        b"ID3",
        1,
        Duration::from_millis(1500),
    );
    let (target, tls) = StreamTarget::parse(&dest).unwrap();
    let output = Output::stream(&dest, 2, &target, tls).unwrap();
    let tone = tone(0.25); // 480 stereo frames per mixer tick
    let silence = vec![0.; tone.len()];
    let start = Instant::now();
    for tick in 0..300 {
        // The only tone plays from 750 ms to 1 s, while the handshake is pending.
        let block = if (75..100).contains(&tick) {
            &tone
        } else {
            &silence
        };
        output.push(block);
        let next = start + BLOCK_DURATION * (tick + 1);
        thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    let mut connection = connections
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_else(|err| panic!("{err}: {:?}", output.status()));
    assert_eq!(output.state.dropped.load(Relaxed), 0, "{:?}", output.status());
    // Decode the first two seconds the server actually received (96 kbit/s).
    let mut mp3 = vec![0; 2 * 96_000 / 8];
    mp3[..3].copy_from_slice(b"ID3");
    connection.read_exact(&mut mp3[3..]).unwrap();
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("received.mp3");
    fs::write(&path, mp3).unwrap();
    let last_loud_frame = decode_audio(&path)
        .chunks_exact(2)
        .rposition(|frame| frame[0].abs() > 0.1)
        .expect("the tone played during the handshake should be audible");
    // Allow 50 ms for MP3 encoder delay.
    assert!(
        last_loud_frame.abs_diff(48_000) < 2400,
        "tone ended at frame {last_loud_frame}"
    );
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
        "Sal-A---Möte-2026-11-05-09-04"
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
        "2026-11-05 09-04 - Källa-1"
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

#[test]
fn reports_recording_folder_failure_without_returning_a_saved_path() {
    let folder = tempfile::tempdir().unwrap();
    let file = folder.path().join("file");
    fs::write(&file, b"existing content").unwrap();
    let settings = Recording {
        folder: file.join("subfolder").display().to_string(),
        format: "WAV".into(),
        ..Recording::default()
    };
    let recording = Output::record(&settings, "Sal", 1);
    wait_for_shutdown();
    assert!(recording.finished());
    assert!(
        recording
            .recording_result()
            .unwrap_err()
            .starts_with("Kunde inte skapa inspelningsfilen i ")
    );
    assert_eq!(fs::read(file).unwrap(), b"existing content");
}

#[test]
#[ignore = "requires FFmpeg"]
fn finalizes_queued_stereo_audio_and_reports_its_full_duration() {
    let folder = tempfile::tempdir().unwrap();
    let settings = Recording {
        folder: folder.path().display().to_string(),
        format: "WAV".into(),
        quality: 24,
        ..Recording::default()
    };
    let mut recording = Output::record(&settings, "Sal", 2);
    let input: Vec<f32> = tone(0.25)
        .into_iter()
        .flat_map(|left| [left, -2.0 * left])
        .collect();
    for _ in 0..12 {
        recording.push(&input);
    }
    recording.stop();
    wait_for_shutdown();
    assert!(recording.finished());
    let path = recording.recording_result().unwrap();

    let decoded = decode_audio(&path);
    assert_eq!(decoded.len(), 12 * input.len());
    for (actual, expected) in decoded.iter().zip(input.iter().cycle()) {
        assert!((actual - expected).abs() < 1e-6);
    }
    let reported = recording.encoded_time();
    assert!(
        reported.abs_diff(Duration::from_millis(240)) <= Duration::from_micros(21),
        "FFmpeg reported {reported:?} for 240 ms of audio"
    );
}
