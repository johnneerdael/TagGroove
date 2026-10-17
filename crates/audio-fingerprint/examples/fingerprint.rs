//! Developer verification tool. The installed app calls the library directly.
use opentag_audio_fingerprint::{backend_info, Control, Job};

fn main() {
    for argument in std::env::args_os().skip(1) {
        if argument == "--info" {
            let info = backend_info();
            println!(
                "{}",
                serde_json::json!({"chromaprint":info.chromaprint,"ffmpeg":info.ffmpeg,"license":info.license,"audio_decoders":info.audio_decoders,"non_audio_decoders":info.non_audio_decoders,"protocols":info.protocols})
            );
            continue;
        }
        let result = Job::new(argument.into(), Control::default()).and_then(Job::run);
        let value = match result {
            Ok(value) => {
                serde_json::json!({"ok":true,"duration":value.duration,"fingerprint":value.encoded,"processed_frames":value.processed_frames})
            }
            Err(error) => serde_json::json!({"ok":false,"error":error.to_string()}),
        };
        println!("{value}");
    }
}
