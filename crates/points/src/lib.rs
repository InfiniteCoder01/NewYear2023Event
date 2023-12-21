use fast_firestore::{ApiV1, DB};
use scheduler::*;

pub async fn get_firestore() -> Option<DB> {
    log_error!(
        "Failed to connect to firestore: {:?}!";
        DB::connect("firebase-private.json".to_owned(), "infinitecoderwebsite".to_owned()).await
    )
}

pub async fn give(uid: &str, amount: u64) {
    let mut firestore = try_map!(get_firestore().await, Some);
    if let Ok(mut doc) = firestore.get_document(&format!("/users/{uid}")).await {
        doc.json["points"] = (doc.json["points"].as_u64().unwrap_or(0) + amount).into();
        try_log!(
            "Failed to update user points: {:?}!";
            doc.update(&mut firestore).await
        );
    } else {
        try_log!(
            "Failed to create user in firestore: {:?}!";
            firestore
                .create_document("", "users", uid, &json::object! {
                    "points": amount
                })
                .await
        );
    }
}
