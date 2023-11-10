mod streamer;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Private {
    key: String,
}

fn main() {
    let private: Private =
        toml::from_str(&std::fs::read_to_string("private.toml").unwrap()).unwrap();
    streamer::stream(
        // 1920,
        // 1080,
        // 1280,
        // 720,
        854,
        480,
        20,
        &format!("rtmp://a.rtmp.youtube.com/live2/{}", private.key),
    );
}
