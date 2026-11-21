#![cfg(feature = "fuzzing")]

use motessandare::fuzzing::{
    fuzz_audio_pipeline, fuzz_icecast_stats, fuzz_recording_filename, fuzz_settings_load,
    fuzz_stream_target,
};

#[test]
fn icecast_stats_regression_corpus_and_edges() {
    let seeds: [&[u8]; 3] = [
        include_bytes!("../fuzz/corpus/fuzz_icecast_stats/single_mount.json"),
        include_bytes!("../fuzz/corpus/fuzz_icecast_stats/multi_mount.json"),
        include_bytes!("../fuzz/corpus/fuzz_icecast_stats/empty_icestats.json"),
    ];
    for seed in seeds {
        fuzz_icecast_stats(seed);
    }

    fuzz_icecast_stats(b"");
    fuzz_icecast_stats(b"null");
    fuzz_icecast_stats(b"{not json");
    fuzz_icecast_stats(br#"{"icestats":{"server_id":"\u0007\u001b","source":[]}}"#);
    fuzz_icecast_stats(br#"{"icestats":{"source":[{"listenurl":"not-a-url","listeners":999999999999999999999999}]}}"#);

    let mut oversized = br#"{"icestats":{}}"#.to_vec();
    oversized.resize(1024 * 1024 + 32, b' ');
    fuzz_icecast_stats(&oversized);
}

#[test]
fn settings_load_regression_corpus_and_edges() {
    let modern = include_bytes!("../fuzz/corpus/fuzz_settings_load/modern.json");
    let legacy = include_bytes!("../fuzz/corpus/fuzz_settings_load/legacy_array.json");

    for payload in [
        modern.as_slice(),
        legacy.as_slice(),
        b"{}",
        b"[]",
        b"invalid",
    ] {
        for creds in [
            b"".as_slice(),
            br#"["secret"]"#,
            br#"[["legacy"]]"#,
            br#"{"credentials":[{"id":1,"server":"https://stream.example.org","mount":"/main.opus","user":"source","password":"pw"}]}"#,
            b"not json",
        ] {
            fuzz_settings_load(
                payload,
                creds,
                "https://user:secret@stream.example.org:8443/live?token=abc#frag",
            );
        }
    }

    let duplicate_ids = br#"{
        "destinations": [
            {"id": 0, "mount": "/a"},
            {"id": 1, "mount": "/b"},
            {"id": 1, "mount": "/c"}
        ],
        "sources": [
            {"id": 0, "destination_ids": [1, 1, 999]},
            {"id": 2, "destination_ids": [1, 2]}
        ]
    }"#;
    fuzz_settings_load(duplicate_ids, b"", "http://localhost:8000");
    fuzz_settings_load(duplicate_ids, b"", "ftp://user:secret@example.org/path");
    fuzz_settings_load(duplicate_ids, b"", "");
}

#[test]
fn stream_target_regression_edges() {
    let cases = [
        (
            "https://stream.example.org:8443",
            "/Möte live.opus",
            "source",
            "long-password-1234",
            "Opus",
            96,
            2,
        ),
        (
            "http://[::1]/unused?token=1#frag",
            "///live.mp3",
            " ",
            "another-secret-pw",
            "MP3",
            128,
            1,
        ),
        (
            "ftp://example.org",
            "/live",
            "bad:user",
            "secret-password",
            "Opus",
            96,
            1,
        ),
        ("https://", "/?", "a b", "secret-password", "FLAC", 0, 0),
        ("", "", "\0", "pw", "", u32::MAX, u16::MAX),
    ];

    for (server, mount, user, password, codec, bitrate, channels) in cases {
        fuzz_stream_target(server, mount, user, password, codec, bitrate, channels);
    }
}

#[test]
fn recording_filename_regression_edges() {
    let templates = [
        "",
        "   ",
        "{datum} {tid} - {namn}",
        "{år}_{månad}_{dag}_{timme}{minut}{sekund}_{unix}_{namn}",
        "../../../etc/passwd",
        "...",
        "CON",
        "a\0b\nc:d*e?f\"g<h>i|j\\k/l",
        &"x".repeat(500),
    ];
    let names = ["", "Sal A / Möte", "ÅÄÖ åäö", "a/b\\c:d", &"y".repeat(200)];
    let formats = ["WAV", "FLAC", "Opus", "MP3", "", "unknown"];
    let timestamps = [0, 1_793_869_447, u64::MAX / 2, u64::MAX];

    for template in templates {
        for name in names {
            for format in formats {
                for &ts in &timestamps {
                    fuzz_recording_filename(template, name, format, ts);
                }
            }
        }
    }
}

#[test]
fn audio_pipeline_regression_edges() {
    let f32_edge: &[f32] = &[
        0.0,
        -0.0,
        0.5,
        -0.5,
        1.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
    ];
    let f32_fill = vec![0.25_f32; 2048];
    let i16_edge: &[i16] = &[i16::MIN, -16_384, 0, 16_384, i16::MAX];

    fuzz_audio_pipeline(
        2,
        128,
        48_000,
        0,
        Some(1),
        &[f32_edge, &f32_fill],
        &[i16_edge],
        &[1, 16, 480, u16::MAX],
    );
    fuzz_audio_pipeline(0, 0, 0, 99, Some(0), &[&[0.1]], &[&[1]], &[0, 10]);
    fuzz_audio_pipeline(
        32,
        u16::MAX,
        u32::MAX,
        15,
        None,
        &[&f32_fill],
        &[],
        &[240, 240, 240],
    );
}
