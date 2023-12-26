use scheduler::*;
use std::sync::Mutex;

struct State {
    source: String,
    duration: Duration,
    started: bool,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn load(source: &str) {
    init_logger();
    restart_async_server(async {
        let routes = make_minimal_server(points::make_leaderboard_server());
        routes
    });

    // let background = background.clone();
    // let source = source.to_owned();
    // std::thread::spawn(move || {
    //     background.set_file_source(&source);
    // });

    let file = try_log!("Failed to load media file {:?}: {}", source; std::fs::File::open(source));
    let size = try_log!("Failed to get size of file {:?}: {}", source; file.metadata()).len();
    let reader = std::io::BufReader::new(file);
    let mp4 = try_log!("Failed to get header of media file {:?}: {}", source; mp4::Mp4Reader::read_header(reader, size));

    *STATE.lock().unwrap() = Some(State {
        source: source.to_owned(),
        duration: try_log!("Invalid duration of media file {:?}: {}", source; Duration::from_std(mp4.duration())),
        started: false,
    });
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
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();
    if !state.started && time_left < state.duration - Duration::milliseconds(500) {
        let source = state.source.clone();
        let background = background.clone();
        std::thread::spawn(move || {
            background.set_file_source(&source);
        });
        state.started = true;
    }
    if time_left < Duration::zero() {
        let background = background.clone();
        std::thread::spawn(move || {
            background.disable_background_video();
        });
        false
    } else {
        true
    }
}
