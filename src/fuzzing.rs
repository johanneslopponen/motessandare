use std::collections::HashSet;

use reqwest::StatusCode;

use crate::{
    Destination, Source, capture,
    engine::{self, ChannelMeter, Converter},
    listener_stats, output,
    settings::{self, StoredSettings, ThemePreference, credentials},
};

pub fn fuzz_icecast_stats(data: &[u8]) {
    let _ = listener_stats::parse(data);
    let _ = listener_stats::probe_server_id_from_json(data);

    let bounded = &data[..data.len().min(2 * 1024 * 1024)];
    for status in [
        StatusCode::OK,
        StatusCode::UNAUTHORIZED,
        StatusCode::FORBIDDEN,
        StatusCode::NOT_FOUND,
        StatusCode::SERVICE_UNAVAILABLE,
    ] {
        let _ = listener_stats::read_response(status, bounded);
    }
}

pub fn fuzz_settings_load(settings_json: &[u8], credential_json: &[u8], server_url: &str) {
    if let Ok(stored) = serde_json::from_slice::<StoredSettings>(settings_json) {
        let (mut destinations, mut raw_sources, theme) = match stored {
            StoredSettings::Object(raw) => (raw.destinations, raw.sources, raw.theme),
            StoredSettings::LegacyArray(sources) => {
                (Vec::new(), sources, ThemePreference::System)
            }
        };

        let _ =
            credentials::restore_passwords(credential_json, &mut destinations, &mut raw_sources);
        let settings = settings::migrate_raw(destinations, raw_sources, theme);

        let mut dest_ids = HashSet::with_capacity(settings.destinations.len());
        for dest in &settings.destinations {
            assert_ne!(dest.id, 0, "migrated destination id must be non-zero");
            assert!(
                dest_ids.insert(dest.id),
                "migrated destination ids must be unique"
            );
        }

        let mut source_ids = HashSet::with_capacity(settings.sources.len());
        for source in &settings.sources {
            assert_ne!(source.id, 0, "migrated source id must be non-zero");
            assert!(
                source_ids.insert(source.id),
                "migrated source ids must be unique"
            );

            let mut linked = HashSet::with_capacity(source.destination_ids.len());
            for &dest_id in &source.destination_ids {
                assert!(
                    dest_ids.contains(&dest_id),
                    "source references unknown destination id {dest_id}"
                );
                assert!(
                    linked.insert(dest_id),
                    "source contains duplicate destination id {dest_id}"
                );
            }
        }

        let next_dest = settings.next_destination_id();
        assert!(next_dest >= 1);
        assert!(!dest_ids.contains(&next_dest));

        let next_source = settings.next_source_id();
        assert!(next_source >= 1);
        assert!(!source_ids.contains(&next_source));
    }

    let dest = Destination {
        server: server_url.to_owned(),
        ..Destination::default()
    };
    if let Some(cleaned) = dest.server_without_secrets() {
        let rerun = Destination {
            server: cleaned,
            ..Destination::default()
        };
        assert_eq!(rerun.server_without_secrets(), None);
    }
}

pub fn fuzz_stream_target(
    server: &str,
    mount: &str,
    user: &str,
    password: &str,
    codec: &str,
    bitrate: u32,
    channels: u16,
) {
    let _ = output::username_hint(user);

    let dest = Destination {
        id: 1,
        name: "Fuzz".into(),
        server: server.to_owned(),
        mount: mount.to_owned(),
        user: user.to_owned(),
        password: password.to_owned(),
        codec: codec.to_owned(),
        bitrate,
    };

    if let Ok((target, tls)) = output::StreamTarget::parse(&dest) {
        let url = target.server_url(tls);
        assert!(url.starts_with(if tls { "https://" } else { "http://" }));
        let normalized_mount = target.mount();
        assert!(normalized_mount.starts_with('/'));
        assert!(!normalized_mount.contains(['?', '#']));
        assert!(!normalized_mount.chars().any(char::is_control));

        if let Ok((command, _)) = output::stream_command(&dest, channels.clamp(1, 2), &target, tls)
            && password.len() >= 8
        {
            for arg in command.get_args() {
                assert!(
                    !arg.to_string_lossy().contains(password),
                    "ffmpeg command args must never contain password"
                );
            }
        }
    }
}

pub fn fuzz_recording_filename(template: &str, name: &str, format: &str, unix_secs: u64) {
    let dt = output::utc_from_unix(unix_secs);
    assert!((1..=12).contains(&dt.month));
    assert!((1..=31).contains(&dt.day));
    assert!(dt.hour < 24);
    assert!(dt.minute < 60);
    assert!(dt.second < 60);

    let stem = output::format_recording_stem(template, name, dt);
    assert!(!stem.is_empty());
    assert!(stem.chars().count() <= 128);
    assert!(!stem.chars().any(|ch| {
        ch.is_control() || matches!(ch, '/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*')
    }));
    assert!(!stem.starts_with('.') && !stem.ends_with('.'));
    assert_eq!(stem.trim(), stem);

    let preview = output::preview_recording_filename(template, name, format);
    assert!(preview.contains('.'));
    assert!(
        !preview
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\'))
    );
}

#[allow(clippy::too_many_arguments)]
pub fn fuzz_audio_pipeline(
    channels: u8,
    min_frames: u16,
    sample_rate: u32,
    left: usize,
    right: Option<usize>,
    f32_chunks: &[&[f32]],
    i16_chunks: &[&[i16]],
    read_counts: &[u16],
) {
    let channels = usize::from(channels).clamp(1, 16);
    let min_frames = usize::from(min_frames).clamp(1, 1024);
    let rate = sample_rate.clamp(8_000, 192_000);

    let reader = capture::fuzz_reader(channels, min_frames, rate);
    for chunk in f32_chunks.iter().take(16) {
        capture::fuzz_push_f32(&reader, chunk);
    }
    for chunk in i16_chunks.iter().take(16) {
        capture::fuzz_push_i16(&reader, chunk);
    }
    for &count in read_counts.iter().take(16) {
        if let Some((samples, out_ch, out_rate)) = reader.read(usize::from(count)) {
            assert_eq!(out_ch, channels);
            assert_eq!(out_rate, rate);
            assert_eq!(samples.len() % channels, 0);
            assert!(samples.iter().all(|s| s.is_finite()));
        }
    }
    if let Some((samples, _, _)) = reader.read(usize::MAX) {
        assert_eq!(samples.len() % channels, 0);
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    let conv_reader = capture::fuzz_reader(channels, 8192, rate);
    for chunk in f32_chunks.iter().take(16) {
        capture::fuzz_push_f32(&conv_reader, chunk);
    }
    for chunk in i16_chunks.iter().take(16) {
        capture::fuzz_push_i16(&conv_reader, chunk);
    }

    let mut converter = Converter::default();
    let source = Source {
        left,
        right,
        ..Source::default()
    };
    let out_channels = if right.is_some() { 2 } else { 1 };
    let mut meter_l = ChannelMeter::default();
    let mut meter_r = ChannelMeter::default();

    for _ in 0..8 {
        if let Some(block) = converter.pull(&conv_reader) {
            assert_eq!(block.channels, channels);
            assert_eq!(block.samples.len(), engine::FRAMES * channels);

            let (pcm, peak, present) = engine::route(&source, Some(&block));
            assert_eq!(pcm.len(), engine::FRAMES * out_channels);
            assert_eq!(
                present,
                left < channels && right.is_none_or(|r| r < channels)
            );
            meter_l.update(if peak.0.is_finite() { peak.0 } else { 0.0 });
            meter_r.update(if peak.1.is_finite() { peak.1 } else { 0.0 });
        }
    }
}
