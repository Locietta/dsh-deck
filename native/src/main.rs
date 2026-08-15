#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use clap::Parser;
use command_group::{CommandGroup, GroupChild};
use tao::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Window, WindowBuilder},
};
use url::{Host, Url};
use wry::{WebView, WebViewBuilder};

const READY_PREFIX: &str = "dsh web: ";
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(target_os = "windows")]
const WINDOW_CHROME_SCRIPT: &str = include_str!("window-chrome.js");
const LOADING_HTML: &str = r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Starting DSH Deck</title>
<style>
  :root { color-scheme: dark; font: 14px/1.5 system-ui, sans-serif; background: #101318; color: #f2f4f8; }
  body { min-height: 100vh; margin: 0; display: grid; place-items: center; }
  main { width: min(28rem, calc(100vw - 3rem)); }
  h1 { margin: 0 0 .5rem; font-size: 1.25rem; font-weight: 600; }
  p { margin: 0; color: #aeb7c6; }
  .bar { height: 2px; margin-top: 1.25rem; overflow: hidden; background: #293241; }
  .bar::after { content: ""; display: block; width: 40%; height: 100%; background: #4c8fd6; animation: progress 1.1s ease-in-out infinite alternate; }
  .hint { margin-top: 1rem; font-size: .85rem; color: #7d8797; }
  @keyframes progress { to { transform: translateX(150%); } }
  @media (prefers-reduced-motion: reduce) { .bar::after { width: 100%; animation: none; } }
</style>
<main><h1>Starting DSH Deck</h1><p>Waiting for the local DeepSeek Harness runtime.</p><div class="bar"></div><p class="hint" id="first-launch-hint" hidden>Still starting. The first launch can take a few minutes while the operating system inspects the app&#39;s files; later launches are much faster.</p></main>
<script>setTimeout(() => { document.getElementById("first-launch-hint").hidden = false }, 10000)</script>
</html>"#;

#[derive(Debug, Parser)]
#[command(version, about = "Desktop shell for the DeepSeek Harness Web UI")]
struct Args {
    /// Connect to an already-running loopback dsh Web UI.
    #[arg(
        long,
        value_name = "URL",
        conflicts_with_all = ["dsh_root", "runtime"]
    )]
    url: Option<Url>,

    /// Start dsh from a source checkout through pnpm.
    #[arg(
        long,
        env = "DSH_DECK_DSH_ROOT",
        value_name = "PATH",
        conflicts_with_all = ["url", "runtime"]
    )]
    dsh_root: Option<PathBuf>,

    /// pnpm executable used with --dsh-root.
    #[arg(long, default_value = "pnpm", value_name = "PATH")]
    pnpm: PathBuf,

    /// Start the packaged Node and dsh installation at this runtime directory.
    #[arg(
        long,
        env = "DSH_DECK_RUNTIME",
        value_name = "PATH",
        conflicts_with_all = ["url", "dsh_root"]
    )]
    runtime: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WindowAction {
    Drag,
    Minimize,
    ToggleMaximize,
    Close,
}

#[derive(Debug)]
enum UserEvent {
    ServerReady(Url),
    ServerFailed(String),
    WindowAction(WindowAction),
    Tick,
}

fn main() -> Result<()> {
    let args = Args::parse();
    run(args)
}

fn run(args: Args) -> Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let window_builder = WindowBuilder::new()
        .with_title("DSH Deck")
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_min_inner_size(LogicalSize::new(800.0, 520.0))
        .with_visible(false);
    #[cfg(target_os = "windows")]
    let window_builder = window_builder.with_decorations(false);
    let window = window_builder
        .build(&event_loop)
        .context("failed to create the DSH Deck window")?;
    let webview =
        build_webview(&window, proxy.clone()).context("failed to create the system WebView")?;
    window.set_visible(true);

    let mut child = match resolve_initial_target(&args, &proxy)? {
        InitialTarget::Url(url) => {
            let _ = proxy.send_event(UserEvent::ServerReady(url));
            None
        }
        InitialTarget::Child(child) => Some(child),
    };

    let ticker_running = Arc::new(AtomicBool::new(true));
    spawn_ticker(proxy, Arc::clone(&ticker_running));
    let mut runtime_ready = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::UserEvent(UserEvent::ServerReady(url)) => {
                if let Err(error) = validate_loopback_url(&url).and_then(|()| {
                    webview.load_url(url.as_str()).map_err(anyhow::Error::from)
                }) {
                    show_error(&window, &webview, &format!("Cannot open the dsh Web UI: {error:#}"));
                } else {
                    runtime_ready = true;
                }
            }
            Event::UserEvent(UserEvent::ServerFailed(message)) => {
                stop_child(&mut child);
                show_error(&window, &webview, &message);
            }
            Event::UserEvent(UserEvent::WindowAction(action)) => match action {
                WindowAction::Drag => {
                    if let Err(error) = window.drag_window() {
                        eprintln!("dsh-deck: failed to drag window: {error}");
                    }
                }
                WindowAction::Minimize => window.set_minimized(true),
                WindowAction::ToggleMaximize => {
                    let maximized = !window.is_maximized();
                    window.set_maximized(maximized);
                    sync_window_chrome(&webview, maximized);
                }
                WindowAction::Close => {
                    ticker_running.store(false, Ordering::Relaxed);
                    stop_child(&mut child);
                    *control_flow = ControlFlow::Exit;
                }
            },
            Event::UserEvent(UserEvent::Tick) => {
                if let Some(process) = child.as_mut() {
                    match process.try_wait() {
                        Ok(Some(status)) => {
                            child = None;
                            let detail = if runtime_ready {
                                format!("The dsh runtime stopped ({status}). Restart DSH Deck to reconnect.")
                            } else {
                                format!("The dsh runtime exited before the Web UI was ready ({status}).")
                            };
                            show_error(&window, &webview, &detail);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            child = None;
                            show_error(&window, &webview, &format!("Cannot monitor the dsh runtime: {error}"));
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                sync_window_chrome(&webview, window.is_maximized());
                #[cfg(target_os = "linux")]
                if let Err(error) = webview.set_bounds(wry::Rect {
                    position: tao::dpi::PhysicalPosition::new(0, 0).into(),
                    size: size.into(),
                }) {
                    eprintln!("dsh-deck: failed to resize WebView: {error}");
                }
                #[cfg(not(target_os = "linux"))]
                let _ = size;
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                ticker_running.store(false, Ordering::Relaxed);
                stop_child(&mut child);
                *control_flow = ControlFlow::Exit;
            }
            Event::LoopDestroyed => {
                ticker_running.store(false, Ordering::Relaxed);
                stop_child(&mut child);
            }
            _ => {}
        }
    });
}

enum InitialTarget {
    Url(Url),
    Child(GroupChild),
}

fn resolve_initial_target(args: &Args, proxy: &EventLoopProxy<UserEvent>) -> Result<InitialTarget> {
    if let Some(url) = args.url.clone() {
        validate_loopback_url(&url)?;
        return Ok(InitialTarget::Url(url));
    }

    let (mut command, description) = if let Some(root) = args.dsh_root.as_ref() {
        let pnpm = resolve_pnpm(&args.pnpm);
        let mut command = Command::new(&pnpm);
        command
            .arg("--dir")
            .arg(root)
            .args(["dsh", "web", "--port", "0"]);
        (
            command,
            format!("dsh through {} in {}", pnpm.display(), root.display()),
        )
    } else {
        let root = match args.runtime.clone() {
            Some(root) => root,
            None => adjacent_runtime_root()?,
        };
        let (node, dsh) = packaged_runtime_paths(&root)?;
        let mut command = Command::new(&node);
        command.arg(dsh).args(["web", "--port", "0"]);
        if std::env::var_os("NODE_COMPILE_CACHE").is_none()
            && let Some(cache) = node_compile_cache_dir()
        {
            command.env("NODE_COMPILE_CACHE", cache);
        }
        (command, format!("packaged dsh through {}", node.display()))
    };
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut group = command.group();
    // kill_on_drop and creation_flags exist on Windows builders only; Unix
    // relies on the explicit stop_child teardown. command-group writes the
    // final flags while adding CREATE_SUSPENDED, so CREATE_NO_WINDOW must be
    // set on its builder rather than on Command.
    #[cfg(target_os = "windows")]
    {
        group.kill_on_drop(true);
        group.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = group
        .spawn()
        .with_context(|| format!("failed to start {description}"))?;
    let stdout = child
        .inner()
        .stdout
        .take()
        .context("dsh stdout was not piped")?;
    spawn_stdout_reader(stdout, proxy.clone());
    Ok(InitialTarget::Child(child))
}

/// Resolve the default bare `pnpm` name on Windows, where `CreateProcess`
/// appends only `.exe` and misses the `pnpm.cmd` shim that npm and corepack
/// install. Explicitly configured paths are used as given.
fn resolve_pnpm(configured: &Path) -> PathBuf {
    #[cfg(target_os = "windows")]
    if configured.as_os_str() == "pnpm"
        && let Some(path) = std::env::var_os("PATH")
    {
        for directory in std::env::split_paths(&path) {
            for name in ["pnpm.exe", "pnpm.cmd", "pnpm.bat"] {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    configured.to_path_buf()
}

fn adjacent_runtime_root() -> Result<PathBuf> {
    let executable = std::env::current_exe().context("failed to locate the DSH Deck executable")?;
    let directory = executable
        .parent()
        .context("the DSH Deck executable has no parent directory")?;
    let runtime = directory.join("runtime");
    if !runtime.is_dir() {
        bail!(
            "packaged runtime not found at {}; pass --runtime <directory>, --dsh-root <checkout>, or --url <running-loopback-url>",
            runtime.display()
        )
    }
    Ok(runtime)
}

/// Per-user directory for Node's on-disk compile cache. The packaged runtime
/// re-parses and re-compiles its whole module graph on every launch;
/// NODE_COMPILE_CACHE lets Node persist V8 bytecode across launches instead.
/// The bundle directory itself may be read-only, so the cache lives in the
/// platform user cache location; without one the cache is simply disabled.
fn node_compile_cache_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")));
    Some(base?.join("dsh-deck").join("node-compile-cache"))
}

fn packaged_runtime_paths(root: &Path) -> Result<(PathBuf, PathBuf)> {
    let (node, dsh) = packaged_runtime_paths_unchecked(root);
    if !node.is_file() {
        bail!("packaged Node executable not found at {}", node.display())
    }
    if !dsh.is_file() {
        bail!("packaged dsh entry not found at {}", dsh.display())
    }
    Ok((node, dsh))
}

fn packaged_runtime_paths_unchecked(root: &Path) -> (PathBuf, PathBuf) {
    #[cfg(target_os = "windows")]
    let node = root.join("node").join("node.exe");
    #[cfg(not(target_os = "windows"))]
    let node = root.join("node").join("bin").join("node");
    let dsh = root
        .join("dsh")
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js");
    (node, dsh)
}

fn spawn_stdout_reader(
    stdout: impl std::io::Read + Send + 'static,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::spawn(move || {
        let mut ready = false;
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    println!("{line}");
                    if !ready && let Some(url) = parse_ready_url(&line) {
                        ready = true;
                        if proxy.send_event(UserEvent::ServerReady(url)).is_err() {
                            return;
                        }
                    }
                }
                Err(error) => {
                    let _ = proxy.send_event(UserEvent::ServerFailed(format!(
                        "Cannot read dsh startup output: {error}"
                    )));
                    return;
                }
            }
        }
        if !ready {
            let _ = proxy.send_event(UserEvent::ServerFailed(
                "dsh stopped before announcing its Web UI URL.".to_owned(),
            ));
        }
    });
}

fn spawn_ticker(proxy: EventLoopProxy<UserEvent>, running: Arc<AtomicBool>) {
    thread::spawn(move || {
        while running.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(500));
            if proxy.send_event(UserEvent::Tick).is_err() {
                break;
            }
        }
    });
}

fn parse_window_action(message: &str) -> Option<WindowAction> {
    match message {
        "dsh-deck:drag" => Some(WindowAction::Drag),
        "dsh-deck:minimize" => Some(WindowAction::Minimize),
        "dsh-deck:toggle-maximize" => Some(WindowAction::ToggleMaximize),
        "dsh-deck:close" => Some(WindowAction::Close),
        _ => None,
    }
}

fn sync_window_chrome(webview: &WebView, maximized: bool) {
    let script = format!(
        "window.__DSH_DECK_SET_MAXIMIZED__?.({})",
        if maximized { "true" } else { "false" }
    );
    if let Err(error) = webview.evaluate_script(&script) {
        eprintln!("dsh-deck: failed to update window controls: {error}");
    }
}

fn build_webview(window: &Window, proxy: EventLoopProxy<UserEvent>) -> wry::Result<WebView> {
    let builder = WebViewBuilder::new()
        .with_html(LOADING_HTML)
        .with_devtools(cfg!(debug_assertions));

    #[cfg(target_os = "windows")]
    let builder = builder
        .with_initialization_script(WINDOW_CHROME_SCRIPT)
        .with_ipc_handler(move |request| {
            if let Some(action) = parse_window_action(request.body()) {
                let _ = proxy.send_event(UserEvent::WindowAction(action));
            }
        });
    #[cfg(not(target_os = "windows"))]
    let _ = proxy;

    #[cfg(target_os = "linux")]
    {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder.build_gtk(window.gtk_window())
    }
    #[cfg(not(target_os = "linux"))]
    {
        builder.build(window)
    }
}

fn parse_ready_url(line: &str) -> Option<Url> {
    let raw = line
        .strip_prefix(READY_PREFIX)?
        .split_ascii_whitespace()
        .next()?;
    Url::parse(raw)
        .ok()
        .filter(|url| validate_loopback_url(url).is_ok())
}

fn validate_loopback_url(url: &Url) -> Result<()> {
    if !matches!(url.scheme(), "http" | "https") {
        bail!("only http(s) dsh URLs are supported")
    }
    let loopback = match url.host() {
        Some(Host::Domain(name)) => name.eq_ignore_ascii_case("localhost"),
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    };
    if !loopback {
        bail!("the MVP accepts loopback dsh URLs only")
    }
    Ok(())
}

fn show_error(window: &Window, webview: &WebView, message: &str) {
    window.set_title("DSH Deck — runtime unavailable");
    let escaped = escape_html(message);
    let html = format!(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>DSH Deck runtime unavailable</title><style>:root{{color-scheme:dark;font:14px/1.5 system-ui,sans-serif;background:#101318;color:#f2f4f8}}body{{min-height:100vh;margin:0;display:grid;place-items:center}}main{{width:min(36rem,calc(100vw - 3rem));padding:1.5rem;border:1px solid #293241;background:#181d25}}h1{{margin:0 0 .75rem;font-size:1.25rem}}p{{margin:0;color:#d9a03f;white-space:pre-wrap}}</style><main><h1>Runtime unavailable</h1><p>{escaped}</p></main></html>"#
    );
    if let Err(error) = webview.load_html(&html) {
        eprintln!("dsh-deck: {message}\ndsh-deck: failed to render the error page: {error}");
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn stop_child(child: &mut Option<GroupChild>) {
    if let Some(mut process) = child.take() {
        if let Err(error) = process.kill() {
            eprintln!("dsh-deck: failed to stop dsh runtime: {error}");
            return;
        }
        if let Err(error) = process.wait() {
            eprintln!("dsh-deck: failed to reap dsh runtime: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_canonical_dsh_readiness_line() {
        let url = parse_ready_url("dsh web: http://127.0.0.1:43121")
            .expect("canonical readiness line should parse");
        assert_eq!(url.as_str(), "http://127.0.0.1:43121/");
    }

    #[test]
    fn ignores_lan_and_unrelated_lines() {
        assert!(parse_ready_url("dsh web: http://192.168.1.5:3080").is_none());
        assert!(parse_ready_url("building dsh web").is_none());
    }

    #[test]
    fn accepts_loopback_hosts_only() {
        for raw in [
            "http://127.0.0.1:3080",
            "http://localhost:3080",
            "http://[::1]:3080",
        ] {
            validate_loopback_url(&Url::parse(raw).expect("test URL should parse"))
                .expect("loopback URL should be accepted");
        }
        assert!(
            validate_loopback_url(
                &Url::parse("https://example.com").expect("test URL should parse")
            )
            .is_err()
        );
    }

    #[test]
    fn escapes_runtime_errors_for_the_local_error_page() {
        assert_eq!(
            escape_html("<bad> & \"worse\""),
            "&lt;bad&gt; &amp; &quot;worse&quot;"
        );
    }

    #[test]
    fn packaged_runtime_uses_the_bundle_layout() {
        let root = Path::new("runtime-root");
        let (node, dsh) = packaged_runtime_paths_unchecked(root);
        #[cfg(target_os = "windows")]
        assert_eq!(node, root.join("node").join("node.exe"));
        #[cfg(not(target_os = "windows"))]
        assert_eq!(node, root.join("node").join("bin").join("node"));
        assert_eq!(
            dsh,
            root.join("dsh")
                .join("node_modules")
                .join("@deepseek-ai")
                .join("dsh")
                .join("lib")
                .join("bin.js")
        );
    }

    #[test]
    fn accepts_only_owned_window_actions() {
        assert_eq!(
            parse_window_action("dsh-deck:drag"),
            Some(WindowAction::Drag)
        );
        assert_eq!(
            parse_window_action("dsh-deck:toggle-maximize"),
            Some(WindowAction::ToggleMaximize)
        );
        assert_eq!(parse_window_action("close"), None);
        assert_eq!(parse_window_action("dsh-deck:unknown"), None);
    }
}
