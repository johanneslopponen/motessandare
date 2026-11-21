#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input {
    channels: u8,
    min_frames: u16,
    sample_rate: u32,
    left: usize,
    right: Option<usize>,
    f32_chunks: Vec<Vec<f32>>,
    i16_chunks: Vec<Vec<i16>>,
    read_counts: Vec<u16>,
}

fuzz_target!(|input: Input| {
    let f32_slices: Vec<&[f32]> = input
        .f32_chunks
        .iter()
        .take(16)
        .map(|chunk| &chunk[..chunk.len().min(1024)])
        .collect();
    let i16_slices: Vec<&[i16]> = input
        .i16_chunks
        .iter()
        .take(16)
        .map(|chunk| &chunk[..chunk.len().min(1024)])
        .collect();
    let read_counts = &input.read_counts[..input.read_counts.len().min(16)];

    motessandare::fuzzing::fuzz_audio_pipeline(
        input.channels,
        input.min_frames,
        input.sample_rate,
        input.left,
        input.right,
        &f32_slices,
        &i16_slices,
        read_counts,
    );
});
