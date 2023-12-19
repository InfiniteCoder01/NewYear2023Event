pub mod game;
pub mod tetromino;

use crate::game::Game;
use batbox_la::*;
use scheduler::*;

static STATE: std::sync::Mutex<Option<State>> = std::sync::Mutex::new(None);
const GAME_SIZE: vec2<usize> = vec2(10, 20);

pub struct State {
    games: Vec<Game>,
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn load(args: &str) {
    init_logger();
    log::info!("Got args: {args:?}");
    restart_async_server(async {
        let routes = make_dev_server("tetro", &|uid, websocket| async move {
            use futures_util::stream::StreamExt;
            use futures_util::SinkExt;
            use warp::filters::ws::Message;
            let (mut tx, mut rx) = websocket.split();

            macro_rules! try_send {
                ($message: expr) => {
                    if let Err(err) = tx.send($message).await {
                        log::debug!("Send error: {err}");
                        return;
                    }
                };
            }

            {
                let success = {
                    let mut state = STATE.lock().unwrap();
                    let state = state.as_mut().unwrap();
                    if state.games.len() > 50 {
                        false
                    } else if state.games.iter().any(|game| game.uid == uid) {
                        true
                    } else {
                        state.games.push(Game::new(
                            GAME_SIZE,
                            uid.clone(),
                            "TODO: Names".to_owned(),
                        ));
                        true
                    }
                };
                if !success {
                    try_send!(Message::text("!Queue is full, try again later."));
                    return;
                }
            }

            // log::info!("{name} joined");

            loop {
                if let Some(position) = {
                    let mut state = STATE.lock().unwrap();
                    let state = state.as_mut().unwrap();
                    state.games.iter().position(|game| game.uid == uid)
                } {
                    if position < 2 {
                        tx.send(Message::text("You're in!")).await.ok();
                        break;
                    }

                    tx.send(Message::text(format!("Position in queue: {position}")))
                        .await
                        .ok();
                } else {
                    tx.send(Message::text(
                        "!You're not in queue, something went horribly wrong.",
                    ))
                    .await
                    .ok();
                    return;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            }

            let send_uid = uid.clone();
            tokio::spawn(async move {
                let uid = send_uid;
                loop {
                    let message = {
                        let state = STATE.lock().unwrap();
                        let state = state.as_ref().unwrap();

                        if let Some(game) = state.games.iter().take(2).find(|game| game.uid == uid)
                        {
                            let mut message = Vec::new();
                            message.extend_from_slice(&(game.board.size.x as u32).to_le_bytes());
                            message.extend_from_slice(&(game.board.size.y as u32).to_le_bytes());
                            for tile in game.board.field.iter() {
                                message
                                    .extend_from_slice(&tile.map_or(0, color_to_u32).to_le_bytes());
                            }
                            message.extend_from_slice(&game.zone_meter.to_le_bytes());
                            message.extend_from_slice(&game.zone_max.to_le_bytes());
                            message.extend_from_slice(
                                &(game.board.zone_lines.len() as u32).to_le_bytes(),
                            );
                            for line in &game.board.zone_lines {
                                message.extend_from_slice(&line.clone().move_by(0.0).to_le_bytes());
                            }

                            message.extend_from_slice(&game.tetromino.pos.x.to_le_bytes());
                            message.extend_from_slice(&game.tetromino.pos.y.to_le_bytes());
                            message.extend_from_slice(&(game.tetromino.size as u32).to_le_bytes());
                            message.extend_from_slice(
                                &color_to_u32(game.tetromino.color).to_le_bytes(),
                            );
                            message.extend_from_slice(
                                &(game.tetromino.blocks.len() as u32).to_le_bytes(),
                            );
                            for block in &game.tetromino.blocks {
                                message.push(block.x);
                                message.push(block.y);
                            }

                            Some(message)
                        } else {
                            None
                        }
                    };

                    if let Some(message) = message {
                        if let Err(err) = tx.send(Message::binary(message)).await {
                            log::debug!("Send error: {err}");
                            break;
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                }
            });
            tokio::spawn(async move {
                while let Some(Ok(message)) = rx.next().await {
                    if let Ok(command) = message.to_str() {
                        let mut state = STATE.lock().unwrap();
                        let state = state.as_mut().unwrap();
                        if let Some(game) =
                            state.games.iter_mut().take(2).find(|game| game.uid == uid)
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
                        }
                    }

                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
            });
        });
        routes
    });

    let mut state = STATE.lock().unwrap();
    *state = Some(State { games: Vec::new() });
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

    if state.games.len() > 2 && state.games[0].uid == "AI" && state.games[1].uid == "AI" {
        state.games.drain(..2);
    } else {
        while state.games.len() < 2 {
            state.games.push(Game::new(
                GAME_SIZE,
                "AI".to_owned(),
                "Builtin AI".to_owned(),
            ))
        }
    }

    if let Some([game1, game2]) = &mut state.games.get_mut(..2) {
        let tile = (height / game1.board.size.y.max(game2.board.size.y) as f64)
            .min(width / (game1.board.size.x.max(game2.board.size.x) + 3) as f64 / 2.0);

        let board1_size = game1.board.size.map(|x| x as f64) * tile + vec2(tile * 3.0, 0.0);
        let board2_size = game2.board.size.map(|x| x as f64) * tile + vec2(tile * 3.0, 0.0);
        let padding = (width - (board1_size.x + board2_size.y)) / 3.0;

        // * Frames
        let mut lost = !game1.frame(
            &context,
            tile,
            vec2(padding + tile * 3.0, (height - board1_size.y) / 2.0),
            Some(game2),
        );
        lost |= !game2.frame(
            &context,
            tile,
            vec2(
                board1_size.x + padding * 2.0 + tile * 3.0,
                (height - board2_size.y) / 2.0,
            ),
            Some(game1),
        );
        if lost {
            state.games.drain(..2);
        }
    }

    true
}
