//! WebKitGTK's media backend accepts HTTP, HTTPS and blob, not custom schemes.
//! Serve only opaque playback grants on an ephemeral IPv4 loopback port.
use crate::{library::LibraryState, playback::AudioStream};
use gtk::{gio, glib};
use soup::prelude::*;
use std::{
    io::Read,
    sync::{Arc, Mutex},
};

pub(crate) struct PlaybackEndpoint(pub u16);
impl PlaybackEndpoint {
    pub(crate) fn source(
        &self,
        source: crate::contracts::PlaybackSource,
    ) -> crate::contracts::PlaybackSource {
        crate::contracts::PlaybackSource {
            url: source.url.replacen(
                "soundshelf-audio://localhost/",
                &format!("http://127.0.0.1:{}/audio/", self.0),
                1,
            ),
        }
    }
}

pub(crate) fn install(
    window: &tauri::WebviewWindow,
    state: Arc<LibraryState>,
) -> Result<PlaybackEndpoint, Box<dyn std::error::Error>> {
    let (server, endpoint) = start(state)?;
    // GTK retains this callback and the listener until the main window closes.
    gtk::prelude::WidgetExt::connect_destroy(&window.gtk_window()?, move |_| {
        soup::prelude::ServerExt::disconnect(&server);
    });
    Ok(endpoint)
}

fn start(
    state: Arc<LibraryState>,
) -> Result<(soup::Server, PlaybackEndpoint), Box<dyn std::error::Error>> {
    let server: soup::Server = glib::Object::new();
    let address =
        gio::InetSocketAddress::new(&gio::InetAddress::new_loopback(gio::SocketFamily::Ipv4), 0);
    server.listen(&address, soup::ServerListenOptions::empty())?;
    let port = server
        .uris()
        .first()
        .map(|uri| uri.port())
        .filter(|port| *port > 0 && *port <= u16::MAX as i32)
        .ok_or_else(|| std::io::Error::other("Playback listener unavailable."))?
        as u16;
    // Audio requests never use a body; do not accumulate client-supplied bytes.
    server.connect_request_started(|_, message| {
        if let Some(body) = message.request_body() {
            body.set_accumulate(false);
        }
    });
    // request-read supports asynchronous responses without the soup3 0.5
    // add_handler binding's null-query-table warnings. A status bypasses routing.
    server.connect_request_read(move |server, message| {
        let Some(uri) = message.uri() else {
            message.set_status(400, None);
            return;
        };
        let headers = message.request_headers();
        let host = headers.as_ref().and_then(|headers| headers.one("Host"));
        let origin = headers.as_ref().and_then(|headers| headers.one("Origin"));
        if !allowed_request(
            host.as_deref(),
            origin.as_deref(),
            &uri.path(),
            uri.query().is_some(),
            port,
        ) {
            message.set_status(403, None);
            return;
        }
        let url = format!("soundshelf-audio://localhost/{}", &uri.path()[7..]);
        let method = message.method().unwrap_or_default().to_string();
        let range = headers
            .and_then(|headers| headers.one("Range"))
            .map(|value| value.to_string());
        let state = Arc::clone(&state);
        let server = server.clone();
        let message = message.clone();
        message.set_status(200, None);
        server.pause_message(&message);
        glib::MainContext::default().spawn_local(async move {
            let result = tauri::async_runtime::spawn_blocking(move || {
                state.audio_response(&url, &method, range.as_deref())
            })
            .await;
            let Ok(audio) = result else {
                message.set_status(503, None);
                server.unpause_message(&message);
                return;
            };
            message.set_status(audio.status, None);
            if let Some(headers) = message.response_headers() {
                for (name, value) in audio.headers {
                    headers.replace(name, &value);
                }
            }
            let Some(body) = message.response_body() else {
                server.unpause_message(&message);
                return;
            };
            body.set_accumulate(false);
            let Some(stream) = audio.stream.filter(|_| audio.length > 0) else {
                body.complete();
                server.unpause_message(&message);
                return;
            };
            let reader = Arc::new(Mutex::new(Some(stream)));
            let next_reader = Arc::clone(&reader);
            let next_server = server.clone();
            message.connect_wrote_chunk(move |message| {
                next_server.pause_message(message);
                read_next(
                    next_server.clone(),
                    message.clone(),
                    Arc::clone(&next_reader),
                );
            });
            // Release the descriptor when the client cancels, including while paused.
            let cleanup = Arc::clone(&reader);
            message.connect_finished(move |_| {
                if let Ok(mut stream) = cleanup.lock() {
                    stream.take();
                }
            });
            read_next(server, message, reader);
        });
    });
    Ok((server, PlaybackEndpoint(port)))
}

fn allowed_request(
    host: Option<&str>,
    origin: Option<&str>,
    path: &str,
    query: bool,
    port: u16,
) -> bool {
    host == Some(format!("127.0.0.1:{port}").as_str())
        && origin.is_none_or(|value| {
            value == "tauri://localhost"
                || (cfg!(debug_assertions) && value == "http://localhost:1420")
        })
        && !query
        && path
            .strip_prefix("/audio/")
            .is_some_and(|token| token.len() == 32 && token.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn read_next(
    server: soup::Server,
    message: soup::ServerMessage,
    reader: Arc<Mutex<Option<AudioStream>>>,
) {
    glib::MainContext::default().spawn_local(async move {
        let result = tauri::async_runtime::spawn_blocking(move || {
            let mut guard = reader
                .lock()
                .map_err(|_| std::io::Error::other("Playback unavailable."))?;
            let Some(stream) = guard.as_mut() else {
                return Ok(Vec::new());
            };
            let mut bytes = vec![0; crate::playback::READ_CHUNK];
            let count = stream.read(&mut bytes)?;
            bytes.truncate(count);
            Ok::<_, std::io::Error>(bytes)
        })
        .await;
        match result {
            Ok(Ok(bytes)) => {
                if let Some(body) = message.response_body() {
                    if bytes.is_empty() {
                        body.complete();
                    } else {
                        body.append_bytes(&glib::Bytes::from_owned(bytes));
                    }
                }
            }
            _ => {
                if let Some(socket) = message.socket() {
                    let _ = socket.close();
                }
            }
        }
        server.unpause_message(&message);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loopback_route_rejects_remote_hosts_origins_and_arbitrary_paths() {
        let path = "/audio/0123456789abcdef0123456789abcdef";
        assert!(allowed_request(
            Some("127.0.0.1:43123"),
            None,
            path,
            false,
            43123
        ));
        assert!(allowed_request(
            Some("127.0.0.1:43123"),
            Some("tauri://localhost"),
            path,
            false,
            43123
        ));
        assert_eq!(
            allowed_request(
                Some("127.0.0.1:43123"),
                Some("http://localhost:1420"),
                path,
                false,
                43123
            ),
            cfg!(debug_assertions)
        );
        for host in [
            None,
            Some("localhost:43123"),
            Some("evil.example:43123"),
            Some("127.0.0.1:43124"),
        ] {
            assert!(!allowed_request(host, None, path, false, 43123));
        }
        assert!(!allowed_request(
            Some("127.0.0.1:43123"),
            Some("https://evil.example"),
            path,
            false,
            43123
        ));
        for invalid in [
            "/audio/../secret",
            "/audio/%2e%2e",
            "/etc/passwd",
            "/audio/abcd",
        ] {
            assert!(!allowed_request(
                Some("127.0.0.1:43123"),
                None,
                invalid,
                false,
                43123
            ));
        }
        assert!(!allowed_request(
            Some("127.0.0.1:43123"),
            None,
            path,
            true,
            43123
        ));
    }
}

#[cfg(test)]
mod native_tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};
    use webkit2gtk::{SettingsExt, UserContentManagerExt, WebViewExt};

    // Runs an actual WebKit/GStreamer media pipeline; needs a Linux desktop.
    #[test]
    #[ignore = "requires a desktop session and WebKitGTK media codecs"]
    fn native_webkit_plays_pauses_and_seeks_loopback_audio() {
        gtk::init().expect("GTK desktop session");
        let controller = std::process::Command::new("node")
            .arg("scripts/build-playback-test-module.mjs")
            .current_dir(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .unwrap(),
            )
            .output()
            .expect(
                "Node and installed frontend dependencies are required for the controller probe",
            );
        assert!(
            controller.status.success(),
            "controller bundle: {}",
            String::from_utf8_lossy(&controller.stderr)
        );
        let controller = String::from_utf8(controller.stdout).expect("controller JavaScript");
        let (fixture, state, info, _) = crate::playback::playback_tests::fixture();
        let path = fixture.path().join("audio/nested/track.wav");
        // Ten seconds of silence spans several 64 KiB response blocks.
        let mut wav = std::fs::read(&path).unwrap();
        wav[4..8].copy_from_slice(&160036u32.to_le_bytes());
        wav[40..44].copy_from_slice(&160000u32.to_le_bytes());
        wav.resize(160044, 0);
        std::fs::write(&path, &wav).unwrap();
        for (extension, encoder) in [
            ("mp3", vec!["lamemp3enc"]),
            ("ogg", vec!["vorbisenc", "!", "oggmux"]),
        ] {
            let output = path.with_extension(extension);
            let status = std::process::Command::new("gst-launch-1.0")
                .args([
                    "-q",
                    "filesrc",
                    &format!("location={}", path.display()),
                    "!",
                    "wavparse",
                    "!",
                    "audioconvert",
                    "!",
                ])
                .args(encoder)
                .args(["!", "filesink", &format!("location={}", output.display())])
                .status()
                .unwrap();
            assert!(status.success(), "generate {extension}");
        }
        let originals: Vec<_> = ["wav", "mp3", "ogg"]
            .map(|extension| {
                let path = path.with_extension(extension);
                let bytes = std::fs::read(&path).unwrap();
                (path, bytes)
            })
            .into();
        let library = state.get(&info.id).unwrap();
        crate::scan::begin(&library).unwrap();
        crate::scan::execute(&library).unwrap();
        let assets = crate::library::lock(&state.database)
            .unwrap()
            .query(&crate::contracts::AssetQuery {
                tag_ids: Vec::new(),
                favorites_only: false,
                library_id: info.id.clone(),
                search: String::new(),
                offset: 0,
                limit: 10,
            })
            .unwrap()
            .assets;
        let state = Arc::new(state);
        let (server, endpoint) = start(Arc::clone(&state)).unwrap();
        assert_eq!(assets.len(), 3, "all three codecs must be exercised");
        for asset in assets {
            let expected = originals
                .iter()
                .find(|(path, _)| path.extension().unwrap() == asset.extension.as_str())
                .unwrap()
                .1
                .clone();
            let source = endpoint.source(
                state
                    .prepare_playback(crate::contracts::PlaybackRequest {
                        library_id: info.id.clone(),
                        asset_id: asset.id,
                    })
                    .unwrap(),
            );
            exercise_http(endpoint.0, &source.url, expected);
            let origins = if cfg!(debug_assertions) {
                &["http://localhost:1420", "tauri://localhost"][..]
            } else {
                &["tauri://localhost"][..]
            };
            for origin in origins {
                println!("checking {} from {origin}", asset.extension);
                exercise_media(&source.url, origin, &controller);
                println!(
                    "{}: play, pause, resume and seek passed from {origin}",
                    asset.extension
                );
            }
        }
        for (path, original) in originals {
            assert_eq!(std::fs::read(path).unwrap(), original);
        }
        soup::prelude::ServerExt::disconnect(&server);
    }

    fn exercise_http(port: u16, url: &str, expected: Vec<u8>) {
        let path = url.split_once("/audio/").unwrap().1.to_string();
        let worker = std::thread::spawn(move || {
            use std::io::Write;
            let boundary = (expected.len() - 16).min(65530);
            let range = format!("Range: bytes={boundary}-{}\r\n", boundary + 15);
            for (method, suffix, extra, status, length) in [
                ("GET", path.as_str(), "", 200, expected.len()),
                ("GET", path.as_str(), range.as_str(), 206, 16),
                ("GET", path.as_str(), "Range: bytes=-10\r\n", 206, 10),
                ("GET", path.as_str(), "Range: bytes=999999-\r\n", 416, 0),
                ("HEAD", path.as_str(), "", 200, 0),
                ("GET", "00000000000000000000000000000000", "", 403, 0),
                (
                    "GET",
                    path.as_str(),
                    "Origin: https://evil.example\r\n",
                    403,
                    0,
                ),
            ] {
                let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
                socket
                    .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                    .unwrap();
                write!(socket, "{method} /audio/{suffix} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n{extra}\r\n").unwrap();
                let mut response = Vec::new();
                socket.read_to_end(&mut response).unwrap();
                let boundary = response
                    .windows(4)
                    .position(|bytes| bytes == b"\r\n\r\n")
                    .unwrap();
                let headers = String::from_utf8_lossy(&response[..boundary]).to_ascii_lowercase();
                assert!(
                    headers.starts_with(&format!("http/1.1 {status}")),
                    "{headers}"
                );
                assert!(!headers.contains("access-control-allow-origin"));
                assert_eq!(response[boundary + 4..].len(), length, "{method} {extra}");
                let body = &response[boundary + 4..];
                if status == 200 && method == "GET" {
                    assert_eq!(body, expected);
                }
                if status == 206 {
                    assert!(headers.contains("content-range: bytes "));
                    let start = if extra.contains("bytes=-10") {
                        expected.len() - 10
                    } else {
                        (expected.len() - 16).min(65530)
                    };
                    assert_eq!(body, &expected[start..start + length]);
                }
            }
        });
        let worker = Rc::new(RefCell::new(Some(worker)));
        let pending = Rc::clone(&worker);
        let main_loop = glib::MainLoop::new(None, false);
        let done = main_loop.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
            if pending.borrow().as_ref().unwrap().is_finished() {
                done.quit();
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        main_loop.run();
        worker
            .borrow_mut()
            .take()
            .unwrap()
            .join()
            .expect("loopback HTTP checks");
    }

    fn exercise_media(url: &str, origin: &str, controller: &str) {
        let manager = webkit2gtk::UserContentManager::new();
        assert!(manager.register_script_message_handler("result"));
        let outcome = Rc::new(RefCell::new(None));
        let result = Rc::clone(&outcome);
        let main_loop = glib::MainLoop::new(None, false);
        let done = main_loop.clone();
        manager.connect_script_message_received(Some("result"), move |_, message| {
            *result.borrow_mut() = message.js_value().map(|value| value.to_string());
            done.quit();
        });
        let webview = webkit2gtk::WebView::with_user_content_manager(&manager);
        webview
            .settings()
            .unwrap()
            .set_media_playback_requires_user_gesture(false);
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        gtk::prelude::ContainerExt::add(&window, &webview);
        gtk::prelude::WidgetExt::show_all(&window);
        let html = format!(
            r#"<meta http-equiv="Content-Security-Policy" content="default-src 'self'; script-src 'unsafe-inline'; media-src http://127.0.0.1:*/audio/"><audio muted></audio><script type="module">
{duration_probe}
{controller}
const a = document.querySelector('audio');
const player = new PlaybackController(a, async () => ({{url: '{url}'}}));
const events = [];
for (const name of ['loadstart', 'durationchange', 'loadedmetadata', 'loadeddata', 'canplay', 'playing', 'pause', 'seeked', 'timeupdate', 'ended', 'error']) {{
  a.addEventListener(name, () => events.push([name, a.duration, a.currentTime, a.readyState, player.getSnapshot().phase]));
}}
let finished = false;
const report = status => {{
  if (finished) return;
  finished = true;
  clearTimeout(deadline);
  window.webkit.messageHandlers.result.postMessage(JSON.stringify({{status, events}}));
}};
const deadline = setTimeout(() => report('playback deadline exceeded'), 12000);
const progress = position => new Promise(resolve => {{
  const check = () => {{
    if (!a.paused && a.currentTime > position) {{
      a.removeEventListener('timeupdate', check);
      resolve();
    }}
  }};
  a.addEventListener('timeupdate', check);
  check();
}});
a.muted = true;
a.onerror = () => report('media error ' + a.error?.code);
const run = async () => {{
  try {{
    // Exercise the actual application controller, including pending play promises.
    void player.select({{libraryId: 'fixture', assetId: 'fixture', name: 'synthetic audio'}});
    await waitForDuration(a, 10);
    await progress(0.1);
    await player.toggle();
    const pausedAt = a.currentTime;
    await new Promise(resolve => setTimeout(resolve, 120));
    if (!a.paused || Math.abs(a.currentTime - pausedAt) > 0.05) return report('pause failed');
    const sought = new Promise(resolve => a.addEventListener('seeked', resolve, {{once: true}}));
    player.seek(5);
    await sought;
    if (Math.abs(a.currentTime - 5) > 0.3) return report('seek failed');
    void player.toggle();
    await progress(5.1);
    await player.toggle();
    const pausedAgain = a.currentTime;
    await new Promise(resolve => setTimeout(resolve, 120));
    if (!a.paused || Math.abs(a.currentTime - pausedAgain) > 0.05) return report('controller pause after seek/resume failed');
    for (let attempt = 0; attempt < 3; attempt++) {{
      player.seek(2 + attempt);
      void player.toggle();
      void player.toggle();
      if (!a.paused) return report('rapid pause failed');
      void player.toggle();
      await progress(2.1 + attempt);
      await player.toggle();
      if (!a.paused || player.getSnapshot().phase !== 'paused') return report('rapid resume/pause failed');
    }}
    player.clear();
    report('ok');
  }} catch (e) {{ report(e.message || e.name); }}
}};
void run();
</script>"#,
            duration_probe = include_str!("../tests/support/webkit-playback.mjs")
        );
        webview.load_html(&html, Some(origin));
        let timeout_loop = main_loop.clone();
        let timeout = glib::timeout_add_seconds_local_once(15, move || timeout_loop.quit());
        main_loop.run();
        if outcome.borrow().is_some() {
            timeout.remove();
        }
        gtk::prelude::GtkWindowExt::close(&window);
        let outcome = outcome.borrow();
        let report: serde_json::Value = serde_json::from_str(
            outcome
                .as_deref()
                .expect("WebKit test did not report before timeout"),
        )
        .expect("WebKit test report JSON");
        println!("media events: {}", report["events"]);
        assert_eq!(report["status"], "ok", "{report}");
    }
}
