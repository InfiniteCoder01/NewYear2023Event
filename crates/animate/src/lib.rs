// use scheduler::*;

// #[no_mangle]
// #[allow(improper_ctypes_definitions)]
// pub extern "C" fn load(_: &str) {
//     init_logger();
//     restart_async_server(async {
//         let routes = make_dev_server(
//             "tetro",
//             queue::make_queue(2, 50, Some(std::time::Duration::from_secs(1)), &socket),
//             points::make_leaderboard_server(),
//         );
//         routes
//     });

//     let mut state = STATE.lock().unwrap();
//     *state = Some(State {
//         game: None,
//         last_frame: std::time::Instant::now(),
//         vs_screen: None,

//         endgame: load_wav("Assets/tetro/endgame.wav"),
//     });
// }

// #[no_mangle]
// #[allow(improper_ctypes_definitions)]
// pub extern "C" fn frame(
//     soloud: &soloud::Soloud,
//     context: cairo::Context,
//     width: f64,
//     height: f64,
//     _time_left: Duration,
// ) -> bool {
//     true
// }