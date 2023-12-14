pub mod streamer;
pub use chrono::DateTime;
pub use chrono::Duration;
use libloading::Library;

#[allow(improper_ctypes_definitions)]
pub type PluginFrame = unsafe extern "C" fn(cairo::Context, f64, f64, Duration) -> bool;
pub struct Plugin<'a> {
    pub library: Library,
    pub frame: libloading::Symbol<'a, PluginFrame>,
}

impl Plugin<'_> {
    pub fn load(path: &str, args: &str) -> Self {
        unsafe {
            let library = Library::new(std::path::Path::new(path).canonicalize().unwrap()).unwrap();
            let load = library.get::<unsafe extern "C" fn(&str)>(b"load").unwrap();
            let frame = (*(&library as *const Library))
                .get::<PluginFrame>(b"frame")
                .unwrap();
            load(args);
            Self { library, frame }
        }
    }
}

pub fn init_logger() {
    simplelog::CombinedLogger::init(vec![simplelog::TermLogger::new(
        log::LevelFilter::Info,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )])
    .unwrap();
}

pub fn make_dev_server<'a, U, F>(
    name: &'a str,
    socket: &'static F,
) -> impl warp::Filter<Extract = impl warp::reply::Reply> + Clone + 'a
where
    F: Fn(String, warp::filters::ws::WebSocket) -> U + Sync + 'static,
    U: std::future::Future<Output = ()> + Send + 'static,
{
    use std::fs::read_to_string;
    use warp::Filter;

    let routes = warp::path::end().map(|| "TODO".to_owned());
    let routes = routes.or(warp::path("account").and(warp::fs::dir("./html/account/")));
    let routes = routes.or({
        let editor = warp::path("editor");
        let editor_page = editor.and(warp::fs::dir("./html/editor/"));
        let editor_pkg = editor
            .and(warp::path("pkg"))
            .and(warp::fs::dir("./crates/web-editor/pkg/"));
        editor_page.or(editor_pkg)
    });

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

    let routes = routes.or(warp::path("connect")
        .and(warp::path(name))
        .and(warp::path::param::<String>())
        .and(warp::ws())
        .map(|uid, ws: warp::ws::Ws| ws.on_upgrade(|ws| socket(uid, ws))));

    routes
}

pub fn restart_async_server<F>(
    server: impl std::future::Future<Output = F> + std::marker::Send + 'static,
) where
    F: warp::Filter + Clone + Send + Sync + 'static,
    F::Extract: warp::reply::Reply,
{
    static RUNTIME: std::sync::Mutex<Option<tokio::runtime::Runtime>> = std::sync::Mutex::new(None);

    let mut runtime = RUNTIME.lock().unwrap();
    *runtime = Some(tokio::runtime::Runtime::new().unwrap());
    runtime.as_ref().unwrap().spawn(async {
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
