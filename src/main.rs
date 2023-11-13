pub mod renderer;
pub mod streamer;

use crate::renderer::*;
use hhmmss::Hhmmss;
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
    let stream_start = std::time::Instant::now();
    let mut frame_index = 0;
    let mut last_render_time = 0;
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
            let uptime = stream_start.elapsed();

            frame.clear(Color::BLACK);
            frame.draw_text(
                10,
                10,
                &format!(
                    "Frame {frame_index}\nUptime: {}\nRendered in {}ms\nMax possible framerate: {:.2}",
                    uptime.hhmmssxxx(),
                    last_render_time / 1000,
                    1_000_000.0 / last_render_time as f32,
                ),
                Color::WHITE,
                30.0,
                &fonts,
            );

            frame.fill_rect(
                frame.width as i32 / 2 + (uptime.as_secs_f32().sin() * 100.0) as i32 - 50,
                frame.height as i32 / 2 - 50,
                100,
                100,
                Color::RED,
            );

            last_render_time = render_start.elapsed().as_micros();
            frame_index += 1;
        },
    );
}
