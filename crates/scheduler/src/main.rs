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
        let timestamp = try_log!(
            "Invalid schedule file, failed to parse timestamp: {}!";
            chrono::NaiveDateTime::parse_from_str(timestamp.trim(), "%d.%m.%Y %H:%M:%S")
            => None
        );
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
    pub fn load() -> Option<Self> {
        Some(Self {
            plugins: try_log!(
                "Failed to load schedule: {}!";
                std::fs::read_to_string("schedule.txt")
                => None
            )
            .lines()
            .filter_map(|line| {
                if line.trim().is_empty() || line.starts_with('#') {
                    return None;
                }
                ScheduledPlugin::parse(line)
            })
            .collect(),
        })
    }

    pub fn get(&self, path: &str) -> Option<&ScheduledPlugin> {
        self.plugins
            .get(self.plugins.iter().position(|plugin| plugin.path == path)?)
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

fn spawn_stdin_channel() -> std::sync::mpsc::Receiver<String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || loop {
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        tx.send(buffer).unwrap();
    });
    rx
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
        fn load(plugin: &ScheduledPlugin) -> Option<Self> {
            Some(Self {
                path: plugin.path.clone(),
                plugin: Plugin::load(&plugin.path, &plugin.args)?,
            })
        }
    }

    let mut schedule_timer = std::time::Instant::now();
    let mut schedule = Schedule::default();
    let mut plugin: Option<LoadedPlugin> = None;

    let stdin_channel = std::sync::Mutex::new(spawn_stdin_channel());
    streamer::stream(
        // (1920, 1080),
        (1280, 720),
        7000, // https://support.google.com/youtube/answer/1722171?hl=en#zippy=%2Cvideo-codec-h%2Cframe-rate%2Cbitrate
        128000,
        &format!("rtmp://a.rtmp.youtube.com/live2/{}", private.key),
        move |context, width, height| {
            if schedule_timer.elapsed().as_secs_f32() > 0.5 {
                schedule_timer = std::time::Instant::now();
                schedule = Schedule::load().unwrap_or_default();
            }

            if let Ok(command) = stdin_channel.lock().unwrap().try_recv() {
                let command = command.trim();
                let (cmd, args) = command.split_once(' ').unwrap_or((command, ""));
                match cmd {
                    "reload" => {
                        if let Some(loaded) = &plugin {
                            if let Some(scheduled) = schedule.get(&loaded.path) {
                                if !scheduled.path.is_empty() {
                                    log::info!("Reloading plugin {}", scheduled.path);
                                    plugin = LoadedPlugin::load(scheduled);
                                }
                            }
                        }
                    }
                    "plugin" => {
                        if let Some(loaded) = &plugin {
                            if let Some(command) = &loaded.plugin.command {
                                unsafe {
                                    command(args);
                                }
                            } else {
                                log::info!(
                                    "Plugin \"{}\" does not implement CLI interface!",
                                    loaded.path
                                )
                            }
                        } else {
                            log::error!("No plugin loaded to execute plugin command!");
                        }
                    }
                    _ => log::error!("{cmd}: not a valid command!"),
                }
            }

            if let Some(loaded_plugin) = &mut plugin {
                let next = schedule.get_next(&loaded_plugin.path);
                if !unsafe {
                    (loaded_plugin.plugin.frame)(
                        context,
                        width,
                        height,
                        next.map_or(Duration::zero(), |next| next.timestamp - Local::now()),
                    )
                } {
                    plugin = next.and_then(LoadedPlugin::load);
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
                log_error!("{}"; context.show_text("Nothing is scheduled!"));
            }
            if plugin.is_none() {
                if let Some(scheduled) = schedule.get_scheduled() {
                    if !scheduled.path.is_empty() {
                        log::info!("Loading plugin {}", scheduled.path);
                        plugin = LoadedPlugin::load(scheduled);
                    }
                }
            }
        },
        std::env::args().collect::<Vec<_>>()[1..] != ["Pi"],
    );
}
