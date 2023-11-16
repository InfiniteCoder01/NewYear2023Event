pub mod streamer;

use hhmmss::Hhmmss;
use mixr::stream::AudioStream;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Private {
    key: String,
}

fn main() {
    let private: Private =
        toml::from_str(&std::fs::read_to_string("private.toml").unwrap()).unwrap();
    let mut song =
        mixr::stream::Stream::from_file("Assets/Feint - We Won't Be Alone (feat. Laura Brehm).wav")
            .unwrap();

    let stream_start = std::time::Instant::now();
    let mut frame_index = 0;
    let mut last_frame = std::time::Instant::now();
    let mut frame_times = [0u128; 30];
    streamer::stream(
        // (1920, 1080),
        (1280, 720),
        // (640, 480),
        30,
        5950, // https://support.google.com/youtube/answer/1722171?hl=en#zippy=%2Cvideo-codec-h%2Cframe-rate%2Cbitrate
        44100,
        128000,
        &format!("rtmp://a.rtmp.youtube.com/live2/{}", private.key),
        move |context, width, height, audio| {
            let frame_time = last_frame.elapsed();
            last_frame = std::time::Instant::now();
            frame_times[frame_index % frame_times.len()] = frame_time.as_micros();
            let frame_time = frame_times.iter().copied().sum::<u128>() as usize / frame_times.len();
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
                .show_text(&format!("Frame time is {}ms", frame_time / 1000))
                .unwrap();
            context.move_to(20.0, 90.0);
            context
                .show_text(&format!(
                    "Framerate: {:.2}",
                    1_000_000.0 / frame_time as f32,
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

            // * Audio
            if audio.get_voice_state(0).unwrap() != mixr::PlayState::Playing {
                let buffer = audio
                    .create_buffer(
                        mixr::BufferDescription {
                            format: song.format(),
                        },
                        Some(&song.get_pcm().unwrap()),
                    )
                    .unwrap();
                audio
                    .play_buffer(buffer, 0, mixr::PlayProperties::default())
                    .unwrap();
            }
            println!("Frame!");
        },
    );
}
