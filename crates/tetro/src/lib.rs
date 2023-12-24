pub mod game;
pub mod tetromino;

use crate::game::Game;
use batbox_la::*;
use scheduler::*;
use tween::Tweener;
use warp::filters::ws::{Message, WebSocket};

type BoxedTween<Value, Time> = Tweener<Value, Time, Box<dyn tween::Tween<Value> + Send + Sync>>;

pub struct VSScreen {
    player1: vec2<BoxedTween<f64, f64>>,
    player2: vec2<BoxedTween<f64, f64>>,
}

pub struct State {
    game: Option<[Game; 2]>,
    last_frame: std::time::Instant,
    vs_screen: Option<VSScreen>,
}

static STATE: std::sync::Mutex<Option<State>> = std::sync::Mutex::new(None);
const GAME_SIZE: vec2<usize> = vec2(10, 20);

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn load(_: &str) {
    init_logger();
    restart_async_server(async {
        let routes = make_dev_server(
            "tetro",
            queue::make_queue(2, 50, Some(std::time::Duration::from_secs(1)), &socket),
            points::make_leaderboard_server(),
        );
        routes
    });

    let mut state = STATE.lock().unwrap();
    *state = Some(State {
        game: None,
        last_frame: std::time::Instant::now(),
        vs_screen: None,
    });
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frame(
    soloud: &soloud::Soloud,
    context: cairo::Context,
    width: f64,
    height: f64,
    _time_left: Duration,
) -> bool {
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();
    let frame_time = state.last_frame.elapsed().as_secs_f64();
    state.last_frame = std::time::Instant::now();

    context.select_font_face(
        "Purisa",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );

    if let (Some(vs_screen), Some([game1, game2])) = (&mut state.vs_screen, &state.game) {
        let player1 = vec2(
            vs_screen.player1.x.move_by(frame_time),
            vs_screen.player1.y.move_by(frame_time),
        );
        let player2 = vec2(
            vs_screen.player2.x.move_by(frame_time),
            vs_screen.player2.y.move_by(frame_time),
        );

        context.set_font_size(height / 10.0);
        if let Some(offset) = text_center_offset(&context, "VS") {
            context.set_source_rgb(1.0, 1.0, 1.0);
            context.move_to(width / 2.0 - offset.x, height / 2.0 - offset.y);
            log_error!("{}"; context.show_text("VS"));
        }

        context.set_font_size(height / 20.0);
        context.set_source_rgb(0.96, 0.33, 0.33);
        context.move_to(player1.x, player1.y);
        log_error!("{}"; context.show_text(&game1.name));

        context.set_source_rgb(0.18, 0.38, 1.0);
        context.move_to(player2.x, player2.y);
        log_error!("{}"; context.show_text(&game2.name));

        if vs_screen.player1.x.is_finished()
            && vs_screen.player1.y.is_finished()
            && vs_screen.player2.x.is_finished()
            && vs_screen.player2.y.is_finished()
        {
            state.vs_screen = None;
        }
        return true;
    }

    match queue::get_state() {
        queue::State::Playing
            if match &state.game {
                Some([game1, game2]) => game1.uid == "AI" && game2.uid == "AI",
                None => true,
            } =>
        {
            let mut players = queue::get_players();
            while players.len() < 2 {
                players.push(("AI".to_owned(), "Builtin AI".to_owned()));
            }
            let games = players
                .into_iter()
                .map(|player| Game::new(GAME_SIZE, player.0, player.1))
                .collect::<Vec<_>>();

            context.set_font_size(height / 20.0);
            if let (Some(offset1), Some(offset2)) = (
                text_center_offset(&context, &games[0].name),
                text_center_offset(&context, &games[1].name),
            ) {
                let vs_tween = |value_delta: f64, percent: f32| {
                    value_delta * ((percent * 2.0 - 1.0).powi(7) / 2.0 + 0.5) as f64
                };

                let time = 5.0;

                state.vs_screen = Some(VSScreen {
                    player1: vec2(
                        Tweener::new(-offset1.x * 2.0, width / 2.0, time, Box::new(vs_tween)),
                        Tweener::new(-offset1.y * 2.0, height, time, Box::new(vs_tween)),
                    ),
                    player2: vec2(
                        Tweener::new(
                            width,
                            width / 2.0 - offset2.x * 2.0,
                            time,
                            Box::new(vs_tween),
                        ),
                        Tweener::new(height, -offset2.y * 2.0, time, Box::new(vs_tween)),
                    ),
                });
            }

            state.game = Some(games.try_into().unwrap());
        }
        queue::State::WaitingForPlayers(_) => {
            if state.game.is_none() {
                state.game = Some([
                    Game::new(GAME_SIZE, "AI".to_owned(), "Builtin AI".to_owned()),
                    Game::new(GAME_SIZE, "AI".to_owned(), "Builtin AI".to_owned()),
                ]);
            }
        }
        _ => (),
    }

    if let Some([game1, game2]) = &mut state.game {
        let tile = (height / (game1.board.size.y.max(game2.board.size.y) as f64 + 1.5))
            .min(width / (game1.board.size.x.max(game2.board.size.x) as f64 + 3.0) / 2.0);

        let board1_size = game1.board.size.map(|x| x as f64) * tile + vec2(3.0, 1.5) * tile;
        let board2_size = game2.board.size.map(|x| x as f64) * tile + vec2(3.0, 1.5) * tile;
        let padding = (width - (board1_size.x + board2_size.y)) / 3.0;

        let offset1 = vec2(padding + tile * 3.0, (height - board1_size.y) / 2.0);
        let offset2 = vec2(width / 2.0 + offset1.x, (height - board2_size.y) / 2.0);

        game1.draw(&context, tile, offset1, frame_time);
        game2.draw(&context, tile, offset2, frame_time);

        if let queue::State::Finished(time) = queue::get_state() {
            if time.elapsed() > std::time::Duration::from_secs(10) {
                state.game = None;
                queue::restart();
            }
        } else {
            // * Frames
            let lost1 = !game1.update(soloud, tile, frame_time, Some(game2));
            let lost2 = !game2.update(soloud, tile, frame_time, Some(game1));

            if lost1 || lost2 {
                if lost1 {
                    game1.game_over(tile);
                } else {
                    game1.won(tile);
                }

                if lost2 {
                    game2.game_over(tile);
                } else {
                    game2.won(tile);
                }

                if game1.uid == "AI" && game2.uid == "AI" {
                    state.game = None;
                } else {
                    queue::set_state(queue::State::Finished(std::time::Instant::now()));
                }
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

        if let Some([game1, game2]) = &state.game {
            log::info!("Skipping game between {} and {}!", game1.name, game2.name);
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

    let reciever_uid = uid.clone();
    let reciever = tokio::spawn(async move {
        let uid = reciever_uid;
        while let Some(Ok(message)) = rx.next().await {
            if let Ok(command) = message.to_str() {
                let mut state = STATE.lock().unwrap();
                let state = state.as_mut().unwrap();

                if let Some(game) = state
                    .game
                    .as_mut()
                    .and_then(|games| games.iter_mut().take(2).find(|game| game.uid == uid))
                {
                    match command {
                        "CCW" => game.try_turn(true),
                        "CW" => game.try_turn(false),
                        "Left" => game.try_move(-1),
                        "Right" => game.try_move(1),
                        "Zone" => game.zone(),
                        "FastFall" => game.speedup(true),
                        "SlowFall" => game.speedup(false),
                        _ => (),
                    };
                } else {
                    return;
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    });

    tokio::spawn(async move {
        macro_rules! try_send {
                ($message: expr) => {
                    try_log!("Send error: {}"; tx.send($message).await)
                };
            }

        loop {
            let (message, terminate) = {
                let state = STATE.lock().unwrap();
                let state = state.as_ref().unwrap();

                if let Some(game) = state
                    .game
                    .as_ref()
                    .and_then(|games| games.iter().take(2).find(|game| game.uid == uid))
                {
                    if game.state == game::State::GameOver {
                        (Message::text(format!("You lost :( But don't be disappointed! You've played well and got {} christmas decorations!", game.points)), true)
                    } else if game.state == game::State::Won {
                        (
                            Message::text(format!(
                                "Celebrate, because you won! You've got {} christmas decorations!",
                                game.points
                            )),
                            true,
                        )
                    } else {
                        (Message::binary(game.build_message()), false)
                    }
                } else {
                    reciever.abort();
                    break;
                }
            };

            try_send!(message);
            if terminate {
                break;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
        }
        log::info!("{name} left.");
    });
}
