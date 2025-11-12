use std::path::Path;

use piper_rs::synth::PiperSpeechSynthesizer;
use rodio::buffer::SamplesBuffer;
use std::env;

// static CONFIG_PATH: &str = "./speakers/en_GB-aru-medium.onnx.json";
static CONFIG_PATH: &str = "./speakers/en_US-amy-medium.onnx.json";
static SPEAKER_ID: &i64 = &4;

fn main() {
    let path = env::current_dir().unwrap();
    println!("The current directory is {}", path.display());

    let model = piper_rs::from_config_path(Path::new(CONFIG_PATH))
        .expect("Failed to load model from config path");
    model.set_speaker(*SPEAKER_ID);
    let synth = PiperSpeechSynthesizer::new(model).unwrap();
    let mut samples: Vec<f32> = Vec::new();
    let audio = synth
        .synthesize_parallel(String::from("10 seconds Enthrall"), None)
        .unwrap();
    for result in audio {
        samples.append(&mut result.unwrap().into_vec());
    }

    let stream_handle =
        rodio::OutputStreamBuilder::open_default_stream().expect("open default audio stream");
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    let buf = SamplesBuffer::new(1, 22050, samples);
    sink.append(buf);

    sink.sleep_until_end();
}
