use bidivec::BidiVec;
use scheduler::*;
use std::{collections::HashMap, path::Path, sync::Mutex, time::Instant};

const WIDTH: usize = 64;
const HEIGHT: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct Pixel {
    color: Color,
    uid: Option<String>,
}

impl Pixel {
    pub fn new(color: Color, uid: Option<String>) -> Self {
        Self { color, uid }
    }

    pub fn blank() -> Self {
        Self::new((0.0, 0.0, 0.0), None)
    }
}

impl std::fmt::Display for Pixel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(uid) = &self.uid {
            writeln!(
                f,
                "{} {} {} {}",
                self.color.0, self.color.1, self.color.2, uid
            )
        } else {
            writeln!(f)
        }
    }
}

impl std::str::FromStr for Pixel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let [r, g, b, uid] = s.split(' ').collect::<Vec<_>>()[..] {
            Ok(Self {
                color: (
                    r.parse()
                        .map_err(|err| format!("Failed to parse red color component: {err}!"))?,
                    g.parse()
                        .map_err(|err| format!("Failed to parse green color component: {err}!"))?,
                    b.parse()
                        .map_err(|err| format!("Failed to parse blue color component: {err}!"))?,
                ),
                uid: Some(uid.to_string()),
            })
        } else if s.trim().is_empty() {
            Ok(Self::blank())
        } else {
            Err(format!("Invalid pixel format: {:?}!", s))
        }
    }
}

struct State {
    image: BidiVec<Pixel>,
    save_timeout: Instant,
    timeouts: HashMap<String, Instant>,
}

impl State {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            image: BidiVec::with_elem(Pixel::blank(), width, height),
            save_timeout: Instant::now(),
            timeouts: HashMap::new(),
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Option<Self> {
        std::fs::File::open(path).ok().and_then(|file| {
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(file);
            let mut line = String::new();
            log_error!("Failed to read place size: {}!"; reader.read_line(&mut line))?;
            if let [Ok(width), Ok(height)] = line
                .trim()
                .split(' ')
                .map(str::parse::<usize>)
                .collect::<Vec<_>>()[..]
            {
                let mut state = State::new(width, height);

                for (index, pixel) in reader.lines().enumerate() {
                    state.image[(index % width, index / width)] =
                        log_error!("{}"; log_error!("Failed to read pixel: {}!"; pixel)?.parse())?;
                }

                Some(state)
            } else {
                log::error!("Failed to parse place size from {:?}!", line);
                None
            }
        })
    }

    pub fn save(&self, path: impl AsRef<Path>) {
        use std::io::Write;
        let mut writer = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
        log_error!("Failed to write image size: {}"; writeln!(writer, "{} {}", self.image.width(), self.image.height()));
        for pixel in self.image.iter() {
            log_error!("Failed to write pixel: {}"; write!(&mut writer, "{pixel}"));
        }
    }
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn load(_: &str) {
    init_logger();
    restart_async_server(async {
        let routes = make_dev_server("place", socket, points::make_leaderboard_server());
        routes
    });

    *STATE.lock().unwrap() =
        Some(State::load("state/place.txt").unwrap_or_else(|| State::new(WIDTH, HEIGHT)));
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frame(
    _soloud: &soloud::Soloud,
    _background: &streamer::BackgroundController,
    context: cairo::Context,
    width: f64,
    height: f64,
    time_left: Duration,
    last_event: bool,
) -> bool {
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();
    let height =
        height - points::make_bottom_banner(&context, width, height, time_left, last_event);

    if state.image.width() != WIDTH || state.image.height() != HEIGHT {
        state.image.resize(WIDTH, HEIGHT, Pixel::blank());
    }

    let pixel_size = (width / state.image.width() as f64)
        .min(height / state.image.height() as f64)
        .floor();
    let offset = (
        ((width - state.image.width() as f64 * pixel_size) / 2.0).floor(),
        ((height - state.image.height() as f64 * pixel_size) / 2.0).floor(),
    );

    for (index, pixel) in state.image.iter().enumerate() {
        let (x, y) = (index % state.image.width(), index / state.image.width());
        let x = x as f64 * pixel_size + offset.0;
        let y = y as f64 * pixel_size + offset.1;
        context.set_source_rgb(pixel.color.0, pixel.color.1, pixel.color.2);
        context.rectangle(x, y, pixel_size, pixel_size);
        log_error!("{}"; context.fill());
    }

    if state.save_timeout.elapsed() > std::time::Duration::from_secs(60)
        || time_left <= Duration::zero()
    {
        state.save_timeout = Instant::now();
        log_error!("Failed to create a backup: {}!"; std::fs::copy("state/place.txt", "state/place.txt.bak"));
        state.save("state/place.txt");
        if time_left <= Duration::zero() {
            return false;
        }
    }

    true
}

async fn socket(uid: String, websocket: warp::filters::ws::WebSocket) {
    use futures_util::{SinkExt, StreamExt};
    let (mut tx, mut rx) = websocket.split();
    let palette = [
        0x000000, 0x55415f, 0x646964, 0xd77355, 0x508cd7, 0x64b964, 0xe6c86e, 0xdcf5ff,
    ]
    .into_iter()
    .map(color_from_u32)
    .collect::<Vec<_>>();

    let reciever = tokio::spawn(async move {
        loop {
            while let Some(Ok(message)) = rx.next().await {
                if let Ok(command) = message.to_str() {
                    if let [x, y, color] = command.split(' ').collect::<Vec<_>>()[..] {
                        if let (Ok(x), Ok(y), Ok(color)) = (
                            x.parse::<usize>(),
                            y.parse::<usize>(),
                            u32::from_str_radix(color, 16).map(color_from_u32),
                        ) {
                            if palette.contains(&color) {
                                let allowed = {
                                    let mut state = STATE.lock().unwrap();
                                    let state = state.as_mut().unwrap();
                                    if let Some(timeout) = state.timeouts.get(&uid) {
                                        if timeout.elapsed() > std::time::Duration::from_millis(950)
                                        {
                                            state
                                                .timeouts
                                                .insert(uid.clone(), std::time::Instant::now());
                                            true
                                        } else {
                                            false
                                        }
                                    } else {
                                        state
                                            .timeouts
                                            .insert(uid.clone(), std::time::Instant::now());
                                        true
                                    }
                                };
                                if allowed {
                                    STATE.lock().unwrap().as_mut().unwrap().image[(x, y)] =
                                        Pixel::new(color, Some(uid.clone()));
                                }
                            }
                        }
                    }
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });

    tokio::spawn(async move {
        loop {
            let message = {
                let state = STATE.lock().unwrap();
                let state = state.as_ref().unwrap();

                let mut message = Vec::new();
                message.extend_from_slice(&(state.image.width() as u32).to_le_bytes());
                message.extend_from_slice(&(state.image.height() as u32).to_le_bytes());
                for pixel in state.image.iter() {
                    message.extend_from_slice(&color_to_u32(pixel.color).to_le_bytes());
                }
                message
            };
            if tx
                .send(warp::filters::ws::Message::binary(message))
                .await
                .is_err()
            {
                reciever.abort();
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(700)).await;
        }
    });
}
