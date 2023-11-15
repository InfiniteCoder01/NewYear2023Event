pub mod streamer;

use hhmmss::Hhmmss;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Private {
    key: String,
}

fn main() {
    let private: Private =
        toml::from_str(&std::fs::read_to_string("private.toml").unwrap()).unwrap();

    let stream_start = std::time::Instant::now();
    let mut frame_index = 0;
    let mut last_render_time = 0;
    streamer::stream(
        // (1920, 1080),
        (1280, 720),
        // (854, 480),

        // 128000,
        24000,
        &format!("rtmp://a.rtmp.youtube.com/live2/{}", private.key),
        move |context, width, height| {
            let render_start = std::time::Instant::now();
            let uptime = stream_start.elapsed();

            context.set_source_rgb(1.0, 1.0, 1.0);
            context.select_font_face(
                "Purisa",
                cairo::FontSlant::Normal,
                cairo::FontWeight::Normal,
            );
            context.set_font_size(20.0);
            context.move_to(20.0, 30.0);
            context
                .show_text(&format!(
                "Frame {frame_index}\nUptime: {}\nRendered in {}ms\nMax possible framerate: {:.2}",
                uptime.hhmmssxxx(),
                last_render_time / 1000,
                1_000_000.0 / last_render_time as f32,
            ))
                .unwrap();

            context.set_source_rgb(1.0, 0.0, 0.0);
            context.rectangle(
                width / 2.0 + uptime.as_secs_f64().sin() * 100.0 - 50.0,
                height / 2.0 - 50.0,
                100.0,
                100.0,
            );
            context.fill();

            last_render_time = render_start.elapsed().as_micros();
            frame_index += 1;
        },
    );
}
