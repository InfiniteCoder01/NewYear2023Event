use hhmmss::Hhmmss;
use scheduler::*;
use warp::filters::ws::Message;

static mut STREAM_START: Option<std::time::Instant> = None;
static mut FRAME_INDEX: usize = 0;
static mut LAST_FRAME: Option<std::time::Instant> = None;
static mut FRAME_TIMES: [u128; 30] = [0u128; 30];

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn load(args: &str) {
    init_logger();
    log::info!("Got args: {args:?}");
    restart_async_server(async {
        let routes = make_dev_server("tetro", &|_uid, mut websocket| async move {
            use futures_util::stream::StreamExt;
            use futures_util::SinkExt;

            while let Some(message) = websocket.next().await {
                dbg!(message).ok();
                websocket
                    .send(Message::text("Hello, World!"))
                    .await
                    .unwrap();
            }
        });
        routes
    });

    unsafe {
        STREAM_START = Some(std::time::Instant::now());
        FRAME_INDEX = 0;
        LAST_FRAME = Some(std::time::Instant::now());
        FRAME_TIMES = [0u128; 30];
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frame(
    context: cairo::Context,
    width: f64,
    height: f64,
    time_left: Duration,
) -> bool {
    unsafe {
        let frame_time = LAST_FRAME.unwrap().elapsed();
        LAST_FRAME = Some(std::time::Instant::now());
        FRAME_TIMES[FRAME_INDEX % FRAME_TIMES.len()] = frame_time.as_micros();
        let frame_time = FRAME_TIMES.iter().copied().sum::<u128>() as usize / FRAME_TIMES.len();
        let uptime = STREAM_START.unwrap().elapsed();

        context.set_source_rgb(1.0, 1.0, 1.0);
        context.select_font_face(
            "Purisa",
            cairo::FontSlant::Normal,
            cairo::FontWeight::Normal,
        );
        context.set_font_size(20.0);
        context.move_to(20.0, 30.0);
        context.show_text(&format!("Frame {FRAME_INDEX}")).unwrap();
        context.move_to(20.0, 50.0);
        context
            .show_text(&format!("Uptime: {}", uptime.hhmmssxxx()))
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
        context.move_to(20.0, 110.0);
        context
            .show_text(&format!(
                "Time left by the scheduler: {}",
                time_left.hhmmssxxx(),
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

        FRAME_INDEX += 1;
    }
    true
}
