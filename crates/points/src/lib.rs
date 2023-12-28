use std::sync::Mutex;

use firestore::FirestoreDb;
use scheduler::*;
use serde::{Deserialize, Serialize};

// * ----------------------------------- API stuff ---------------------------------- * //
#[derive(Serialize, Deserialize)]
pub struct UserData {
    points: u64,
}

pub async fn get_firestore() -> Option<FirestoreDb> {
    log_error!(
        "Failed to connect to firestore: {}!";
        FirestoreDb::with_options_service_account_key_file(
                firestore::FirestoreDbOptions::new("infinitecoderwebsite".to_owned()),
                "firebase-private.json".into()
        ).await
    )
}

pub async fn give(uid: &str, amount: u64) {
    let firestore = try_map!(get_firestore().await, Some);
    if let Some(mut user) = try_log!(
        "Failed to get user points from firestore: {}!";
        firestore
            .fluent()
            .select()
            .by_id_in("users")
            .obj::<UserData>()
            .one(uid)
            .await
    ) {
        user.points += amount;
        try_log!(
            "Failed to update user points: {}!";
            firestore.fluent()
                .update()
                .fields(["points"])
                .in_col("users")
                .document_id(uid)
                .object(&user)
                .execute()
                .await
        );
    } else {
        try_log!(
            "Failed to create user in firestore: {}!";
            firestore
                .fluent()
                .insert()
                .into("users")
                .document_id(uid)
                .object(&UserData {
                    points: amount,
                })
                .execute()
                .await
        );
    }
}

pub async fn get_leaderboard() -> Vec<(String, String, UserData)> {
    use futures_util::StreamExt;
    let firestore = try_map!(get_firestore().await, Some => Vec::new());

    let mut users = try_log!(
        "Failed to get users from firestore: {}!";
        firestore
            .fluent()
            .list()
            .from("users")
            .stream_all()
            .await
        => Vec::new()
    );

    let mut leaderboard = Vec::new();
    while let Some(doc) = users.next().await {
        let user = try_log!(
            "Failed to deserialize user data: {}!";
            FirestoreDb::deserialize_doc_to::<UserData>(&doc)
            => Vec::new()
        );
        let uid = doc.name.split('/').last().unwrap();
        leaderboard.push((
            uid.to_owned(),
            get_firebase_user(uid.to_owned())
                .await
                .and_then(|user| user.display_name)
                .unwrap_or("Someone".to_owned()),
            user,
        ));
    }

    leaderboard.sort_by_key(|(uid, _, user)| {
        if uid == "GiAIWs311JaKAWwTEkll5LLPKT63" {
            1
        } else {
            -(user.points as i64)
        }
    });
    leaderboard
}

// * ------------------------------------- State ------------------------------------ * //
#[derive(serde::Serialize)]
pub struct LeaderboardItem {
    pub name: String,
    pub points: u64,
    pub highlighted: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum BannerMessage {
    TimeLeft,
    CurrentLeader,
    TryYourself,
    TelegramAd,
    SupportDonate,
}

struct State {
    leaderboard: Vec<(String, String, UserData)>,
    banner_switch_time: std::time::Instant,
    banner_message: BannerMessage,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

// * ------------------------------------ Server ------------------------------------ * //
pub fn make_leaderboard_server() -> &'static impl Fn(Option<String>) -> warp::reply::Json {
    {
        *STATE.lock().unwrap() = Some(State {
            leaderboard: Vec::new(),
            banner_switch_time: std::time::Instant::now(),
            banner_message: BannerMessage::TimeLeft,
        })
    }

    spawn_in_server_runtime(async {
        loop {
            let new_leaderboard = get_leaderboard().await;
            if !new_leaderboard.is_empty() {
                let mut state = STATE.lock().unwrap();
                let state = state.as_mut().unwrap();
                state.leaderboard = new_leaderboard;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        }
    });

    &move |uid| {
        let state = STATE.lock().unwrap();
        let state = state.as_ref().unwrap();
        let leaderboard = state
            .leaderboard
            .iter()
            .map(|(leader_uid, name, user)| LeaderboardItem {
                name: name.clone(),
                points: user.points,
                highlighted: Some(leader_uid) == uid.as_ref(),
            })
            .collect::<Vec<_>>();
        warp::reply::json(&leaderboard)
    }
}

// * ------------------------------------ Banner ------------------------------------ * //
pub fn make_bottom_banner(
    context: &cairo::Context,
    width: f64,
    height: f64,
    time_left: Duration,
) -> f64 {
    let padding = 10.0;
    let radius = 10.0;
    let banner_height = (height / 20.0).floor() + padding * 2.0 + radius * 2.0;
    let y = height - banner_height;

    // * Frame
    rounded_rectangle(
        context,
        padding,
        y + padding,
        width - padding * 2.0,
        banner_height - padding * 2.0,
        radius,
    );

    context.set_source_rgb(0.1, 0.1, 0.1);
    log_error!("{}"; context.fill_preserve());
    context.set_source_rgb(0.25, 0.6, 0.66);
    context.set_line_width(2.0);
    log_error!("{}"; context.stroke());

    // * Message
    let mut state = STATE.lock().unwrap();
    let state = state.as_mut().unwrap();

    {
        use std::time::Duration;
        if state.banner_switch_time.elapsed()
            > match state.banner_message {
                BannerMessage::TimeLeft => Duration::from_secs(20),
                _ => Duration::from_secs(10),
            }
        {
            state.banner_switch_time = std::time::Instant::now();
            state.banner_message = match state.banner_message {
                BannerMessage::TimeLeft => BannerMessage::CurrentLeader,
                BannerMessage::CurrentLeader => BannerMessage::TryYourself,
                BannerMessage::TryYourself => BannerMessage::TelegramAd,
                BannerMessage::TelegramAd => BannerMessage::SupportDonate,
                BannerMessage::SupportDonate => BannerMessage::TimeLeft,
            }
        }
    }

    context.set_font_size(banner_height - padding * 2.0 - radius * 2.0);
    let message = match state.banner_message {
        BannerMessage::TimeLeft => {
            use hhmmss::Hhmmss;

            let days = time_left.num_days();
            let hhmmss = time_left - Duration::days(days);
            format!(
                "Time left to the next event: {}",
                if days > 0 {
                    format!("{} days and {}", days, hhmmss.hhmmss())
                } else {
                    hhmmss.hhmmss()
                }
            )
        }
        BannerMessage::CurrentLeader => format!("Current leader: {}", state.leaderboard[0].1),
        BannerMessage::TryYourself => {
            "Try it yourself at event.infinitecoder.org (Link in description)".to_owned()
        }
        BannerMessage::TelegramAd => {
            "Follow me on Telegram: https://t.me/InfiniteCoder02".to_owned()
        }
        BannerMessage::SupportDonate => {
            context.set_font_size(((banner_height - padding * 2.0 - radius * 2.0) * 0.6).floor());
            "If you like this event and want to see more, you can support me on Patreon via StreamElements (Links in description)".to_owned()
        }
    };

    if let Some(offset) = text_center_offset(context, &message) {
        context.set_source_rgb(1.0, 1.0, 1.0);
        context.move_to(
            padding + radius,
            y + (banner_height / 2.0).floor() - offset.y,
        );
        log_error!("{}"; context.show_text(&message));
    }

    banner_height
}
