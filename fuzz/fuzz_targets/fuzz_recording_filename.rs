#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input {
    template: String,
    name: String,
    format: String,
    unix_secs: u64,
}

fuzz_target!(|input: Input| {
    motessandare::fuzzing::fuzz_recording_filename(
        &input.template,
        &input.name,
        &input.format,
        input.unix_secs,
    );
});
