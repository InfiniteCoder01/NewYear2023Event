use scheduler::*;

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn load(background: &streamer::BackgroundController, source: &str) {
    init_logger();
    restart_async_server(async {
        let routes = make_minimal_server(points::make_leaderboard_server());
        routes
    });

    background.set_file_source(source);
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frame(
    _soloud: &soloud::Soloud,
    background: &streamer::BackgroundController,
    _context: cairo::Context,
    _width: f64,
    _height: f64,
    time_left: Duration,
) -> bool {
    if time_left < Duration::zero() {
        // background.disable_background_video();
        false
    } else {
        true
    }
}
