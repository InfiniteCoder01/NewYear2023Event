pub use chrono::DateTime;
pub use chrono::Duration;
use libloading::Library;
use batbox_la::*;

#[allow(improper_ctypes_definitions)]
pub type PluginFrame = unsafe extern "C" fn(cairo::Context, f64, f64, Duration) -> bool;

#[allow(improper_ctypes_definitions)]
pub type PluginCommand = unsafe extern "C" fn(&str);

pub struct Plugin<'a> {
    pub library: Library,
    pub frame: libloading::Symbol<'a, PluginFrame>,
    pub command: Option<libloading::Symbol<'a, PluginCommand>>,
}

impl Plugin<'_> {
    pub fn load(path: &str, args: &str) -> Option<Self> {
        unsafe {
            let library = try_log!(
                "Failed to load plugin {:?}: {}!",
                path;
                Library::new(try_log!(
                    "Failed to find plugin {:?}: {}!",
                    path;
                    std::path::Path::new(path).canonicalize()
                    => None
                ))
                => None
            );
            let load = try_log!(
                "Invalid plugin {}!";
                library.get::<unsafe extern "C" fn(&str)>(b"load")
                => None
            );
            let frame = try_log!(
                "Invalid plugin {}!";
                (*(&library as *const Library)).get::<PluginFrame>(b"frame")
                => None
            );
            let command = (*(&library as *const Library))
                .get::<PluginCommand>(b"command")
                .ok();
            load(args);
            Some(Self {
                library,
                frame,
                command,
            })
        }
    }
}

pub fn init_logger() {
    if let Err(err) = simplelog::CombinedLogger::init(vec![simplelog::TermLogger::new(
        log::LevelFilter::Info,
        simplelog::ConfigBuilder::new()
            .add_filter_ignore_str("firestore")
            .add_filter_ignore_str("rs-firebase-admin-sdk")
            .add_filter_ignore_str("gcp_auth")
            .build(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )]) {
        log::error!("{}", err);
    }
}

#[macro_export]
macro_rules! log_error {
    ($format: literal $(, $args: expr)*; $result: expr) => {
        $result.map_or_else(
            |err| {
                log::error!($format, $($args,)* err);
                None
            },
            Some,
        )
    };
}

#[macro_export]
macro_rules! try_map {
    ($value: expr, $match: ident $(=> $return: expr)?) => {{
        match $value  {
            $match(x) => x,
            _ => return $($return)?,
        }
    }};
}

#[macro_export]
macro_rules! try_log {
    ($format: literal $(, $args: expr)*; $result: expr $(=> $return: expr)?) => {
        try_map!(log_error!($format $(, $args)*; $result), Some $(=> $return)?)
    };
}

// * ------------------------------------ Server ------------------------------------ * //
static RUNTIME: std::sync::Mutex<Option<tokio::runtime::Runtime>> = std::sync::Mutex::new(None);
pub fn spawn_in_server_runtime<F>(future: F) -> tokio::task::JoinHandle<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    let mut runtime = RUNTIME.lock().unwrap();
    if runtime.is_none() {
        *runtime = Some(tokio::runtime::Runtime::new().unwrap());
    }
    runtime.as_ref().unwrap().spawn(future)
}

pub fn make_dev_server<'a, Socket, FutureSocket, Leaderboard>(
    name: &'a str,
    socket: Socket,
    leaderboard: &'a Leaderboard,
) -> impl warp::Filter<Extract = impl warp::reply::Reply> + Clone + 'a
where
    Socket: Fn(String, warp::filters::ws::WebSocket) -> FutureSocket + Send + Sync + 'static,
    FutureSocket: std::future::Future<Output = ()> + Send + 'static,
    Leaderboard: Fn(Option<String>) -> warp::reply::Json + Sync,
{
    use std::fs::read_to_string;
    use warp::Filter;

    let routes = warp::path::end().map(|| {
        warp::reply::html(
            r#"<head><meta http-equiv="refresh" content="0; url=/controller" /></head>"#,
        )
    });
    let routes = routes.or(warp::path("account").and(warp::fs::dir("./html/account/")));
    let routes = routes.or(warp::path("editor").and(warp::fs::dir("./html/editor/")));
    let routes = routes.or(warp::path("leaderboard").and(
        warp::path::path("api")
            .and(
                warp::path::param::<String>()
                    .and(warp::path::end())
                    .map(|uid| leaderboard(Some(uid)))
                    .or(warp::path::end().map(|| leaderboard(None))),
            )
            .or(warp::fs::dir("./html/leaderboard/")),
    ));

    let routes = routes.or(warp::path("controller").and(
        warp::path::end()
            .map(move || {
                warp::reply::html(
                    read_to_string(format!("./html/controller/{name}/index.html"))
                        .unwrap_or("Controller not found. Please, report!".to_owned())
                        .replace(
                            "<!-- !META -->",
                            &read_to_string("./html/controller/lib/meta.html").unwrap_or(
                                "<script>alert(\"Meta not found. Please, report!\");</script>"
                                    .to_owned(),
                            ),
                        )
                        .replace(
                            "<!-- !NAV -->",
                            &read_to_string("./html/controller/lib/nav.html")
                                .unwrap_or("Nav not found. Please, report!".to_owned()),
                        ),
                )
            })
            .or(warp::fs::dir(format!("./html/controller/{name}/"))),
    ));

    let socket = std::sync::Arc::new(socket);
    routes.or(warp::path("connect")
        .and(warp::path(name))
        .and(warp::path::param::<String>())
        .and(warp::ws())
        .map(move |uid, ws: warp::ws::Ws| {
            let socket = socket.clone();
            ws.on_upgrade(move |ws| socket(uid, ws))
        }))
}

pub fn restart_async_server<F>(
    server: impl std::future::Future<Output = F> + std::marker::Send + 'static,
) where
    F: warp::Filter + Clone + Send + Sync + 'static,
    F::Extract: warp::reply::Reply,
{
    let mut runtime = RUNTIME.lock().unwrap();
    *runtime = Some(tokio::runtime::Runtime::new().unwrap());
    let runtime = runtime.as_ref().unwrap();

    runtime.spawn(async {
        log::info!("Starting server...");
        let routes = server.await;

        let routes = routes.with(warp::log::custom(|info| {
            log::info!(
                "{} {} => {}",
                info.method(),
                info.path(),
                info.status().as_u16(),
            )
        }));

        warp::serve(routes).run(([127, 0, 0, 1], 1480)).await;
    });
}

// * ----------------------------------- Rendering ---------------------------------- * //
pub type Color = (f64, f64, f64);

pub fn color_to_u32(color: Color) -> u32 {
    ((color.0 * 255.0) as u32) << 16 | ((color.1 * 255.0) as u32) << 8 | ((color.2 * 255.0) as u32)
}

pub fn text_center_offset(context: &cairo::Context, text: &str) -> Option<vec2<f64>> {
    context.text_extents(text).ok().map(|extents| {
        vec2(
            extents.width() / 2.0 + extents.x_bearing(),
            extents.height() / 2.0 + extents.y_bearing(),
        )
    })
}

// * ----------------------------------- Firebase ----------------------------------- * //
pub async fn get_firebase_admin(
) -> Option<rs_firebase_admin_sdk::App<rs_firebase_admin_sdk::GcpCredentials>> {
    let firebase_credentials = try_log!(
        "Failed to load firebase credentials: {}";
        std::fs::read_to_string("firebase-private.json")
        => None
    );
    let service_account = try_log!(
        "Failed to load firebase auth data: {}";
        rs_firebase_admin_sdk::CustomServiceAccount::from_json(&firebase_credentials)
        => None
    );
    Some(try_log!(
        "Failed to connect to firebase: {}";
        rs_firebase_admin_sdk::App::live(service_account.into())
        .await
        => None
    ))
}

pub async fn get_firebase_user(uid: String) -> Option<rs_firebase_admin_sdk::auth::User> {
    if let Some(admin) = get_firebase_admin().await {
        use rs_firebase_admin_sdk::auth::FirebaseAuthService;
        log_error!(
            "Error while fetching user: {}!";
            admin
                .auth()
                .get_user(
                    rs_firebase_admin_sdk::auth::UserIdentifiers::builder()
                        .with_uid(uid)
                        .build(),
                )
                .await
        )
        .and_then(|user| {
            if user.is_none() {
                log::error!("Error: User does not exist!");
            }
            user
        })
    } else {
        None
    }
}
