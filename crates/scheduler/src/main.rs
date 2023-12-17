use chrono::{Local, TimeZone};
use scheduler::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Private {
    key: String,
}

#[derive(Debug)]
pub struct ScheduledPlugin {
    path: String,
    args: String,
    timestamp: DateTime<Local>,
}

impl ScheduledPlugin {
    pub fn parse(line: &str) -> Option<Self> {
        let parts = line.split_once('|');
        if parts.is_none() {
            log::warn!("Plugin schedule formatted incorrectly: '{line}'");
        }
        let (timestamp, command) = parts?;
        let (timestamp, command) = (timestamp.trim(), command.trim());
        let (path, args) = command.split_once(' ').unwrap_or((command, ""));
        let timestamp =
            chrono::NaiveDateTime::parse_from_str(timestamp.trim(), "%d.%m.%Y %H:%M:%S").unwrap();
        Some(Self {
            path: path.trim().to_owned(),
            args: args.trim().to_owned(),
            timestamp: Local.from_local_datetime(&timestamp).unwrap(),
        })
    }
}

#[derive(Debug, Default)]
pub struct Schedule {
    plugins: Vec<ScheduledPlugin>,
}

impl Schedule {
    pub fn load() -> Self {
        Self {
            plugins: std::fs::read_to_string("schedule.txt")
                .unwrap()
                .lines()
                .filter_map(|line| {
                    if line.trim().is_empty() || line.starts_with('#') {
                        return None;
                    }
                    ScheduledPlugin::parse(line)
                })
                .collect(),
        }
    }

    pub fn get_next(&self, path: &str) -> Option<&ScheduledPlugin> {
        self.plugins
            .get(self.plugins.iter().position(|plugin| plugin.path == path)? + 1)
    }

    pub fn get_scheduled(&self) -> Option<&ScheduledPlugin> {
        let current_time = Local::now();
        self.plugins
            .iter()
            .rev()
            .find(|plugin| current_time >= plugin.timestamp)
    }
}

fn main() {
    init_logger();
    let private: Private =
        toml::from_str(&std::fs::read_to_string("private.toml").unwrap()).unwrap();

    struct LoadedPlugin<'a> {
        path: String,
        plugin: Plugin<'a>,
    }

    impl LoadedPlugin<'_> {
        fn load(plugin: &ScheduledPlugin) -> Self {
            Self {
                path: plugin.path.clone(),
                plugin: Plugin::load(&plugin.path, &plugin.args),
            }
        }
    }

    let mut schedule_timer = std::time::Instant::now();
    let mut schedule = Schedule::default();
    let mut plugin: Option<LoadedPlugin> = None;

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
                schedule = Schedule::load();
            }

            if let Some(loaded_plugin) = &mut plugin {
                let next = schedule.get_next(&loaded_plugin.path);
                if !unsafe {
                    (loaded_plugin.plugin.frame)(
                        context,
                        width,
                        height,
                        next.map_or(Duration::max_value(), |next| next.timestamp - Local::now()),
                    )
                } {
                    plugin = next.map(LoadedPlugin::load);
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
                if let Some(scheduled) = schedule.get_scheduled() {
                    if !scheduled.path.is_empty() {
                        log::info!("Loading plugin {}", scheduled.path);
                        plugin = Some(LoadedPlugin::load(scheduled));
                    }
                }
            }
        },
        std::env::args().collect::<Vec<_>>() != ["Pi"],
    );
}
