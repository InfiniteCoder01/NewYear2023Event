pub mod renderer;
mod streamer;

use crate::renderer::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Private {
    key: String,
}

fn main() {
    let private: Private =
        toml::from_str(&std::fs::read_to_string("private.toml").unwrap()).unwrap();

    let font = fontdue::Font::from_bytes(
        include_bytes!("../Assets/Roobert-Regular.ttf") as &[u8],
        fontdue::FontSettings::default(),
    )
    .unwrap();
    let fonts = [font];
    let mut frame_index = 0;
    let mut render_time_avg = 0;
    streamer::stream(
        // 1920,
        // 1080,
        1280,
        720,
        // 854,
        // 480,
        30,
        &format!("rtmp://a.rtmp.youtube.com/live2/{}", private.key),
        move |frame| {
            let render_start = std::time::Instant::now(); // ! Profiling

            frame.clear(Color::WHITE);
            frame.draw_text(100, 100, "Hello, World!", Color::RED, 100.0, &fonts);

            let render_time = render_start.elapsed().as_micros();
            render_time_avg += render_time;
            frame_index += 1;
            println!(
                "Frame {frame_index} rendered in {}ms, AVG render time: {}ms",
                render_time,
                render_time_avg / frame_index
            );
        },
    );
}
