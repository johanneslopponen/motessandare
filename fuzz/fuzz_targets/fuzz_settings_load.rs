#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input {
    settings_json: Vec<u8>,
    credential_json: Vec<u8>,
    server_url: String,
}

fuzz_target!(|input: Input| {
    motessandare::fuzzing::fuzz_settings_load(
        &input.settings_json,
        &input.credential_json,
        &input.server_url,
    );
});
