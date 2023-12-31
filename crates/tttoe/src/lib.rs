pub mod game;

use batbox_la::*;
use game::*;
use scheduler::*;
use warp::filters::ws::{Message, WebSocket};

const PLAYERS: usize = 3;
const GAME_SIZE: vec2<usize> = vec2(6, 6);
const STRIDE: usize = 4;

pub struct State {
    game: Option<Game>,
}

static STATE: std::sync::Mutex<Option<State>> = std::sync::Mutex::new(None);

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn load(_: &str) {
    init_logger();
    restart_async_server(async {
        let routes = make_dev_server(
            "tttoe",
            queue::make_queue(
                PLAYERS,
                50,
                Some(std::time::Duration::from_secs(3 /*0*/)),
                &socket,
            ),
            points::make_leaderboard_server(),
        );
        routes
    });

    let mut state = STATE.lock().unwrap();
    *state = Some(State { game: None });
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frame(
    soloud: &soloud::Soloud,
    _: &streamer::BackgroundController,
    context: cairo::Context,
    width: f64,
    height: f64,
    time_left: Duration,
    last_event: bool,
) -> bool {
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();

    context.select_font_face(
        "Purisa",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );

    let height =
        height - points::make_bottom_banner(&context, width, height, time_left, last_event);

    match queue::get_state() {
        queue::State::Playing
            if match &state.game {
                Some(game) => game.players.iter().all(|player| player.uid == "AI"),
                None => true,
            } =>
        {
            let mut players = queue::get_players();
            while players.len() < PLAYERS {
                players.push(("AI".to_owned(), "Builtin AI".to_owned()));
            }
            let players = players
                .into_iter()
                .enumerate()
                .map(|(index, (uid, name))| Player::new(uid, name, Tag::no(index)))
                .collect();

            state.game = Some(Game::new(GAME_SIZE, players));
        }
        queue::State::WaitingForPlayers(_) => {
            if state.game.is_none() {
                state.game = Some(Game::new(
                    GAME_SIZE,
                    (0..PLAYERS)
                        .map(|index| {
                            Player::new("AI".to_owned(), "Builtin AI".to_owned(), Tag::no(index))
                        })
                        .collect(),
                ));
            }
        }
        _ => (),
    }

    if let Some(game) = &mut state.game {
        let tile = (width / (game.board.width() + 6) as f64)
            .min(height / (game.board.height() + 1) as f64)
            .floor();

        let offset = vec2(
            (width - (game.board.width() + 5) as f64 * tile) / 2.0,
            (height - game.board.height() as f64 * tile) / 2.0,
        )
        .map(f64::floor);

        game.draw(&context, tile, offset);

        if let queue::State::Finished(time) = queue::get_state() {
            if time.elapsed() > std::time::Duration::from_secs(5) {
                if time_left < Duration::zero() {
                    kill_async_server();
                    return false;
                }
                state.game = None;
                queue::restart();
            }
        } else {
            // * Frames
            if !game.update(soloud) {
                queue::set_state(queue::State::Finished(std::time::Instant::now()));
            }
        }
    }

    true
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn command(command: &str) {
    if command == "skip" {
        let mut state = STATE.lock().unwrap();
        let state = state.as_mut().unwrap();

        if let Some(game) = &state.game {
            log::info!(
                "Skipping game between {}, {}, {} and {}!",
                game.players[1].name,
                game.players[2].name,
                game.players[3].name,
                game.players[4].name
            );
            state.game = None;
        }
    }
}

fn socket(
    uid: String,
    name: String,
    mut tx: futures_util::stream::SplitSink<WebSocket, Message>,
    mut rx: futures_util::stream::SplitStream<WebSocket>,
) {
    use futures_util::{SinkExt, StreamExt};
    tokio::spawn(async move {
        while {
            let mut state = STATE.lock().unwrap();
            let state = state.as_mut().unwrap();
            state.game.is_none()
        } {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        macro_rules! try_send {
            ($message: expr) => {
                try_log!("Send error: {}"; tx.send($message).await)
            };
        }

        loop {
            let (message, my_turn) = {
                let mut state = STATE.lock().unwrap();
                let state = state.as_mut().unwrap();
                if let Some(game) = &state.game {
                    if let Some(my_turn) = game.players.iter().position(|player| player.uid == uid)
                    {
                        let mut message = game.build_message(&uid);
                        message.push((game.turn == my_turn) as u8);
                        (Message::binary(message), game.turn == my_turn)
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            };

            try_send!(message);

            if my_turn {
                if let Ok(Some(Ok(message))) =
                    tokio::time::timeout(tokio::time::Duration::from_secs(60), rx.next()).await
                {
                    if let Ok(command) = message.to_str() {
                        if let Some((x, y)) = command.split_once(' ') {
                            if let (Ok(x), Ok(y)) = (x.parse(), y.parse()) {
                                let mut state = STATE.lock().unwrap();
                                let state = state.as_mut().unwrap();
                                if let Some(game) = &mut state.game {
                                    game.try_turn(vec2(x, y));
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    let mut state = STATE.lock().unwrap();
                    let state = state.as_mut().unwrap();
                    if let Some(game) = &mut state.game {
                        game.skip_turn();
                    } else {
                        break;
                    }
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        log::info!("{name} left.");
    });
}
