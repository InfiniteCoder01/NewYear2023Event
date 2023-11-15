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
    let mut last_frame = std::time::Instant::now();
    streamer::stream(
        // (1920, 1080),
        // (1280, 720),
        (640, 480),

        // 128000,
        24000,
        &format!("rtmp://a.rtmp.youtube.com/live2/{}", private.key),
        move |context, width, height| {
            let frame_time = last_frame.elapsed();
            last_frame = std::time::Instant::now();
            let uptime = stream_start.elapsed();

            context.set_source_rgb(1.0, 1.0, 1.0);
            context.select_font_face(
                "Purisa",
                cairo::FontSlant::Normal,
                cairo::FontWeight::Normal,
            );
            context.set_font_size(20.0);
            context.move_to(20.0, 30.0);
            context.show_text(&format!("Frame {frame_index}",)).unwrap();
            context.move_to(20.0, 50.0);
            context
                .show_text(&format!("Uptime: {}", uptime.hhmmssxxx(),))
                .unwrap();
            context.move_to(20.0, 70.0);
            context
                .show_text(&format!("Rendered in {}ms", frame_time.as_millis(),))
                .unwrap();
            context.move_to(20.0, 90.0);
            context
                .show_text(&format!(
                    "Framerate: {:.2}",
                    1_000_000.0 / frame_time.as_micros() as f32,
                ))
                .unwrap();

            context.set_source_rgb(0.0, 0.0, 1.0);
            context.rectangle(
                width / 2.0 + uptime.as_secs_f64().sin() * 100.0 - 50.0,
                height / 2.0 - 50.0,
                100.0,
                100.0,
            );
            context.fill().unwrap();

            frame_index += 1;
        },
    );
}
