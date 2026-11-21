#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input {
    server: String,
    mount: String,
    user: String,
    password: String,
    codec: String,
    bitrate: u32,
    channels: u16,
}

fuzz_target!(|input: Input| {
    motessandare::fuzzing::fuzz_stream_target(
        &input.server,
        &input.mount,
        &input.user,
        &input.password,
        &input.codec,
        input.bitrate,
        input.channels,
    );
});
