use scheduler::*;
use warp::Filter;

#[no_mangle]
pub extern "C" fn load() {
    let tr = tokio::runtime::Runtime::new().unwrap();
    tr.spawn(async {
        let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

        warp::serve(hello).run(([127, 0, 0, 1], 3030)).await;
    });
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frame(
    context: cairo::Context,
    width: f64,
    height: f64,
    time_left: Duration,
) -> bool {
    // let stream_start = std::time::Instant::now();
    // let mut frame_index = 0;
    // let mut last_frame = std::time::Instant::now();
    // let mut frame_times = [0u128; 30];

    // let frame_time = last_frame.elapsed();
    // last_frame = std::time::Instant::now();
    // frame_times[frame_index % frame_times.len()] = frame_time.as_micros();
    // let frame_time = frame_times.iter().copied().sum::<u128>() as usize / frame_times.len();
    // let uptime = stream_start.elapsed();

    // context.set_source_rgb(1.0, 1.0, 1.0);
    // context.select_font_face(
    //     "Purisa",
    //     cairo::FontSlant::Normal,
    //     cairo::FontWeight::Normal,
    // );
    // context.set_font_size(20.0);
    // context.move_to(20.0, 30.0);
    // context.show_text(&format!("Frame {frame_index}",)).unwrap();
    // context.move_to(20.0, 50.0);
    // context
    //     .show_text(&format!("Uptime: {}", uptime.hhmmssxxx(),))
    //     .unwrap();
    // context.move_to(20.0, 70.0);
    // context
    //     .show_text(&format!("Frame time is {}ms", frame_time / 1000))
    //     .unwrap();
    // context.move_to(20.0, 90.0);
    // context
    //     .show_text(&format!(
    //         "Framerate: {:.2}",
    //         1_000_000.0 / frame_time as f32,
    //     ))
    //     .unwrap();

    // context.set_source_rgb(0.0, 0.0, 1.0);
    // context.rectangle(
    //     width / 2.0 + uptime.as_secs_f64().sin() * 100.0 - 50.0,
    //     height / 2.0 - 50.0,
    //     100.0,
    //     100.0,
    // );
    // context.fill().unwrap();

    // frame_index += 1;
    if time_left.num_seconds() < 1 {
        let time = 2.0 - time_left.num_milliseconds() as f64 / 500.0;
        let size = 100.0 * (time - 0.5).powi(2) + 25.0;
        context.set_source_rgb(0.0, 0.0, 1.0);
        context.rectangle(
            width / 2.0 - size,
            height / 2.0 - size,
            size * 2.0,
            size * 2.0,
        );
        context.fill().unwrap();
        time_left > Duration::zero()
    } else {
        context.set_source_rgb(0.0, 0.0, 1.0);
        context.rectangle(width / 2.0 - 50.0, height / 2.0 - 50.0, 100.0, 100.0);
        context.fill().unwrap();
        true
    }
}
