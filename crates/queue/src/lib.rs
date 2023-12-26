use futures_util::stream::{SplitSink, SplitStream};
use scheduler::*;
use std::{future::Future, pin::Pin, sync::Mutex};
use warp::filters::ws::{Message, WebSocket};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum State {
    #[default]
    None,
    WaitingForPlayers(std::time::Instant),
    Playing,
    Finished(std::time::Instant),
}

#[derive(Clone, Debug)]
pub struct Queue {
    queue: Vec<(String, String)>,
    current_game: Vec<(String, String)>,
    state: State,
}

impl Queue {
    fn new() -> Self {
        Self {
            queue: Vec::new(),
            current_game: Vec::new(),
            state: State::WaitingForPlayers(std::time::Instant::now()),
        }
    }

    pub const fn empty() -> Self {
        Self {
            queue: Vec::new(),
            current_game: Vec::new(),
            state: State::None,
        }
    }
}

static QUEUE: Mutex<Queue> = Mutex::new(Queue::empty());

pub fn get_state() -> State {
    QUEUE.lock().unwrap().state.clone()
}

pub fn set_state(state: State) {
    QUEUE.lock().unwrap().state = state;
}

pub fn get_players() -> Vec<(String, String)> {
    QUEUE.lock().unwrap().current_game.clone()
}

pub fn restart() {
    *QUEUE.lock().unwrap() = Queue::new();
}

pub fn make_queue<F>(
    required_players: usize,
    max_queue_size: usize,
    wait_time: Option<std::time::Duration>,
    player: &'static F,
) -> impl Fn(String, WebSocket) -> Pin<Box<dyn Future<Output = ()> + Send>>
where
    F: Fn(String, String, SplitSink<WebSocket, Message>, SplitStream<WebSocket>) + Sync + Send,
{
    {
        let mut queue = QUEUE.lock().unwrap();
        *queue = Queue::new();
    }
    spawn_in_server_runtime(async move {
        loop {
            {
                let mut queue = QUEUE.lock().unwrap();
                match queue.state.clone() {
                    State::None => {
                        log::error!("Invalid queue state: None!");
                        *queue = Queue::new();
                    }
                    State::WaitingForPlayers(time) => {
                        if !queue.queue.is_empty() {
                            let players_to_complete = required_players - queue.current_game.len();
                            let players_to_provide = players_to_complete.min(queue.queue.len());
                            let new_players =
                                queue.queue.drain(..players_to_provide).collect::<Vec<_>>();
                            queue.current_game.extend(new_players);
                            if queue.current_game.len() >= required_players {
                                queue.state = State::Playing;
                            }
                        }
                        if queue.current_game.is_empty() {
                            queue.state = State::WaitingForPlayers(std::time::Instant::now())
                        } else if Some(time.elapsed()) >= wait_time {
                            queue.state = State::Playing;
                        }
                    }
                    State::Playing => (),
                    State::Finished(_) => (),
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
    });

    move |uid: String, websocket: warp::filters::ws::WebSocket| {
        Box::pin(async move {
            use futures_util::stream::StreamExt;
            use futures_util::SinkExt;
            let (mut tx, rx) = websocket.split();

            macro_rules! try_send {
                ($message: expr) => {
                    try_log!("Send error: {}"; tx.send($message).await)
                };
            }

            // * Get meta
            let name = {
                get_firebase_user(uid.clone())
                    .await
                    .and_then(|user| user.display_name.map(|name| format!("@{name}")))
                    .unwrap_or("Someone".to_owned())
            };

            // * Join the queue
            let mut position_in_queue = 0;
            {
                let success = {
                    let mut queue = QUEUE.lock().unwrap();
                    if queue.queue.len() > max_queue_size {
                        false
                    } else if queue.queue.iter().any(|(user, _)| user == &uid)
                        || queue.current_game.iter().any(|(user, _)| user == &uid)
                    {
                        true
                    } else {
                        queue.queue.push((uid.clone(), name.clone()));
                        true
                    }
                };
                if !success {
                    try_send!(Message::text("!Queue is full, try again later."));
                    return;
                }
            }

            // * Wait in the queue
            loop {
                #[derive(Clone, Copy, Debug, PartialEq, Eq)]
                enum UserState {
                    InQueue(usize),
                    InGame,
                    Lost,
                }

                let state = {
                    let queue = QUEUE.lock().unwrap();
                    if queue.current_game.iter().any(|(user, _)| user == &uid) {
                        UserState::InGame
                    } else if let Some(position) =
                        queue.queue.iter().position(|(user, _)| user == &uid)
                    {
                        UserState::InQueue(position + 1)
                    } else {
                        UserState::Lost
                    }
                };

                match state {
                    UserState::InQueue(position) => {
                        if position_in_queue != position {
                            try_send!(Message::text(format!("Position in queue: {position}")));
                            position_in_queue = position;
                        }
                    }
                    UserState::InGame => {
                        try_send!(Message::text("Waiting for players..."));
                        break;
                    }
                    UserState::Lost => {
                        try_send!(Message::text(
                            "!Something went horribly wrong, we lost you in our queues!",
                        ));
                        return;
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }

            log::info!("{name} joined!");

            // * Wait for other players
            while {
                let queue = QUEUE.lock().unwrap();
                matches!(queue.state, State::WaitingForPlayers(_))
            } {
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
            }

            try_send!(Message::text("You're in!"));

            player(uid, name, tx, rx);
        })
    }
}
