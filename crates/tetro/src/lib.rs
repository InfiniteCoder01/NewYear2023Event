pub mod game;
pub mod tetromino;

use crate::game::Game;
use batbox_la::*;
use scheduler::*;

static STATE: std::sync::Mutex<Option<State>> = std::sync::Mutex::new(None);

pub struct State {
    games: Option<(Game, Game)>,
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn load(args: &str) {
    init_logger();
    log::info!("Got args: {args:?}");
    restart_async_server(async {
        let routes = make_dev_server("tetro", &|_uid, websocket| async move {
            use futures_util::stream::StreamExt;
            use futures_util::SinkExt;
            use warp::filters::ws::Message;

            let (mut tx, mut rx) = websocket.split();
            tokio::spawn(async move {
                loop {
                    let message = {
                        let state = STATE.lock().unwrap();
                        let state = state.as_ref().unwrap();

                        if let Some((game, _)) = &state.games {
                            let mut message =
                                vec![game.board.size.x as u32, game.board.size.y as _];
                            message.reserve(game.board.field.len());
                            for tile in game.board.field.iter() {
                                message.push(tile.map_or(0, |color| {
                                    ((color.0 * 255.0) as u32) << 16
                                        | ((color.1 * 255.0) as u32) << 8
                                        | ((color.2 * 255.0) as u32)
                                }));
                            }

                            // message.push();

                            Some(message)
                        } else {
                            None
                        }
                    };

                    if let Some(message) = message {
                        let message = message
                            .iter()
                            .flat_map(|x| x.to_le_bytes())
                            .collect::<Vec<_>>();
                        if let Err(err) = tx.send(Message::binary(message)).await {
                            log::error!("Error sending board state: {err}")
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });
            tokio::spawn(async move {
                while let Some(message) = rx.next().await {
                    match message {
                        Ok(message) => {
                            if let Ok(command) = message.to_str() {
                                let mut state = STATE.lock().unwrap();
                                let state = state.as_mut().unwrap();
                                if let Some((game1, game2)) = &mut state.games {
                                    // match command {
                                    //     "CW" => state.game.try_turn(false),
                                    //     "L" => state.game.try_move(-1),
                                    //     "D" => true,
                                    //     "R" => state.game.try_move(1),
                                    //     "CCW" => state.game.try_turn(true),
                                    //     _ => false,
                                    // };
                                    let game = if command.ends_with('1') { game1 } else { game2 };
                                    match &command[..command.len() - 1] {
                                        "CW" => game.try_turn(false),
                                        "L" => game.try_move(-1),
                                        "SPEED" => {
                                            game.speedup(true);
                                            true
                                        }
                                        "SLOW" => {
                                            game.speedup(false);
                                            true
                                        }
                                        "E" => {
                                            game.effect = std::time::Instant::now();
                                            true
                                        }
                                        "R" => game.try_move(1),
                                        "CCW" => game.try_turn(true),
                                        _ => false,
                                    };
                                }
                            }
                        }
                        Err(err) => log::error!("Error recieving message: {err}"),
                    }
                }
            });
        });
        routes
    });

    let mut state = STATE.lock().unwrap();
    *state = Some(State { games: None });
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frame(
    context: cairo::Context,
    width: f64,
    height: f64,
    _time_left: Duration,
) -> bool {
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();

    if state.games.is_none() {
        state.games = Some((Game::new(vec2(10, 20)), Game::new(vec2(10, 20))));
    }

    if let Some((game1, game2)) = &mut state.games {
        let tile = height / game1.board.size.1.max(game2.board.size.1) as f64;
        let board1_size = game1.board.size.map(|x| x as f64) * tile;
        let board2_size = game2.board.size.map(|x| x as f64) * tile;
        let padding = (width - (board1_size.x + board2_size.y)) / 3.0;

        let mut lost = !game1.frame(
            &context,
            tile,
            vec2(padding, (height - board1_size.1) / 2.0),
            Some(game2),
        );
        game2.tetromino.ai(&mut game2.board);
        lost |= !game2.frame(
            &context,
            tile,
            vec2(
                board1_size.x + padding * 2.0,
                (height - board2_size.1) / 2.0,
            ),
            Some(game1),
        );
        if lost {
            state.games = None;
        }
    }

    true
}
