use firestore::FirestoreDb;
use scheduler::*;
use serde::{Deserialize, Serialize};

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

#[derive(serde::Serialize)]
pub struct LeaderboardItem {
    pub name: String,
    pub points: u64,
    pub highlighted: bool,
}

pub fn make_leaderboard_server() -> &'static impl Fn(Option<String>) -> warp::reply::Json {
    use std::sync::Mutex;
    static LEADERBOARD: Mutex<Vec<(String, String, UserData)>> = Mutex::new(Vec::new());

    spawn_in_server_runtime(async {
        loop {
            let new_leaderboard = get_leaderboard().await;
            if !new_leaderboard.is_empty() {
                let mut leaderboard = LEADERBOARD.lock().unwrap();
                *leaderboard = new_leaderboard;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;
        }
    });

    &move |uid| {
        let leaderboard = LEADERBOARD.lock().unwrap();
        let leaderboard = leaderboard
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

    rounded_rectangle(
        context,
        padding,
        y + padding,
        width - padding * 2.0,
        banner_height - padding * 2.0,
        radius,
    );

    context.set_source_rgb(0.25, 0.6, 0.66);
    log_error!("{}"; context.stroke());

    let message = {
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
    };
    if let Some(offset) = text_center_offset(context, &message) {
        context.set_source_rgb(1.0, 1.0, 1.0);
        context.set_font_size(banner_height - padding * 2.0 - radius * 2.0);
        context.move_to(padding + radius, y + banner_height / 2.0 - offset.y);
        log_error!("{}"; context.show_text(&message));
    }

    banner_height
}
