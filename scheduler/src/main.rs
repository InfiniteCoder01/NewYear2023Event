use chrono::{Local, TimeZone};
use scheduler::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Private {
    key: String,
}

fn main() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    let env = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "scheduler=trace".into());
    let fmt = tracing_subscriber::fmt::layer().pretty().with_target(true);
    tracing_subscriber::registry().with(fmt).with(env).init();
    // let subscriber = tracing_subscriber::FmtSubscriber::builder()
    //     .with_max_level(tracing::Level::TRACE)
    //     .finish();

    let private: Private =
        toml::from_str(&std::fs::read_to_string("private.toml").unwrap()).unwrap();

    let mut schedule_timer = std::time::Instant::now();
    let mut schedule = Vec::<(String, DateTime<Local>)>::new();
    let mut plugin: Option<(String, Plugin)> = None;

    streamer::stream(
        // (1920, 1080),
        (1280, 720),
        7000, // https://support.google.com/youtube/answer/1722171?hl=en#zippy=%2Cvideo-codec-h%2Cframe-rate%2Cbitrate
        44100,
        128000,
        &format!("rtmp://a.rtmp.youtube.com/live2/{}", private.key),
        move |context, width, height| {
            if schedule_timer.elapsed().as_secs_f32() > 0.5 {
                schedule_timer = std::time::Instant::now();
                schedule = std::fs::read_to_string("schedule.txt")
                    .unwrap()
                    .lines()
                    .filter_map(|event| {
                        if event.trim().is_empty() {
                            return None;
                        }
                        if let [timestamp, file] = &event.split('|').collect::<Vec<_>>()[..] {
                            let timestamp = chrono::NaiveDateTime::parse_from_str(
                                timestamp.trim(),
                                "%d.%m.%Y %H:%M:%S",
                            )
                            .unwrap();
                            Some((
                                file.trim().to_owned(),
                                Local.from_local_datetime(&timestamp).unwrap(),
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
            }

            if let Some((name, library)) = &mut plugin {
                let time = schedule
                    .get(schedule.iter().position(|(file, _)| name == file).unwrap() + 1)
                    .map(|timestamp| timestamp.1);
                if !unsafe {
                    (library.frame)(
                        context,
                        width,
                        height,
                        time.map_or(Duration::max_value(), |time| time - Local::now()),
                    )
                } {
                    plugin = None;
                }
            } else {
                context.set_source_rgb(1.0, 1.0, 1.0);
                context.select_font_face(
                    "Purisa",
                    cairo::FontSlant::Normal,
                    cairo::FontWeight::Normal,
                );
                context.set_font_size(40.0);
                context.move_to(20.0, 30.0);
                context.show_text("Nothing is scheduled!").unwrap();
            }
            if plugin.is_none() {
                for (file, time) in schedule.iter().rev() {
                    if &Local::now() >= time {
                        if !file.is_empty() {
                            tracing::info!("Loading plugin {}", file);
                            plugin = Some((file.clone(), Plugin::load(file)));
                        }
                        break;
                    }
                }
            }
        },
        true,
    );
}
