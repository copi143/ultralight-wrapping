#[cfg(feature = "surfman")]
surfman::declare_surfman!();

use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use ul_next::config::FontHinting;
use ul_next::{Config, Library, app::App, window::WindowFlags};

use crate::render::renderer_main_wrapper;
use crate::render::{EXIT_RENDERER, RENDER_MUTEX, RENDER_RECV_STAT_COND, RENDER_SEND_TASK_COND};

mod file;
mod gpu;
mod helper;
mod items;
mod js;
mod render;
mod view;

static LIB: OnceLock<Arc<Library>> = OnceLock::new();

fn read_c_string(ptr: *const u8) -> &'static str {
    unsafe {
        assert!(!ptr.is_null());
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).unwrap()
    }
}

struct ArboardClipboard {
    clipboard: arboard::Clipboard,
}

impl ArboardClipboard {
    fn new() -> Self {
        Self {
            clipboard: arboard::Clipboard::new().unwrap(),
        }
    }
}

impl ul_next::platform::Clipboard for ArboardClipboard {
    fn clear(&mut self) {
        let _ = self.clipboard.clear();
    }

    fn read_plain_text(&mut self) -> Option<String> {
        self.clipboard.get_text().ok()
    }

    fn write_plain_text(&mut self, text: &str) {
        let _ = self.clipboard.set_text(text.to_owned());
    }
}

fn open_window(url: &str) {
    let lib = LIB.get().unwrap().clone();

    let config = Config::start()
        .font_hinting(FontHinting::Smooth)
        .build(lib.clone())
        .unwrap();

    let app = App::new(lib, None, Some(config)).unwrap();

    let window = app
        .create_window(
            900,
            600,
            false,
            WindowFlags {
                borderless: false,
                titled: true,
                resizable: true,
                maximizable: true,
                hidden: false,
            },
        )
        .unwrap();

    window.set_title("Basic App");

    let overlay = window
        .create_overlay(window.width(), window.height(), 0, 0)
        .unwrap();

    overlay.view().load_url(url).unwrap();

    window.set_resize_callback(move |_window, width, height| {
        overlay.resize(width, height);
    });

    let app = Rc::new(app);
    let app_clone = app.clone();

    window.set_close_callback(move |_window| {
        app_clone.quit();
    });

    app.run();
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_open_window(url: *const u8) {
    let url = read_c_string(url).to_string();
    std::thread::spawn(move || {
        open_window(&url);
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_init() {
    unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_client_init() {
    LIB.set(Library::linked())
        .map_err(|_| "Library already initialized")
        .unwrap();
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_server_init() {}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_into_render_main() {
    renderer_main_wrapper();
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_wait_render_init() {
    let mut lock = RENDER_MUTEX.lock();
    while lock.is_none() {
        RENDER_RECV_STAT_COND.wait_for(&mut lock, Duration::from_millis(100));
    }
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_exit() {
    EXIT_RENDERER.store(true, Ordering::SeqCst);
    RENDER_SEND_TASK_COND.notify_one();

    let mut lock = RENDER_MUTEX.lock();
    while lock.is_some() {
        RENDER_RECV_STAT_COND.wait_for(&mut lock, Duration::from_millis(100));
    }
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_alloc(size: usize) -> usize {
    unsafe extern "C" {
        fn malloc(size: usize) -> *mut u8;
    }
    unsafe { malloc(size) as usize }
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_free(ptr: usize) {
    unsafe extern "C" {
        fn free(ptr: *mut u8);
    }
    unsafe { free(ptr as *mut u8) }
}
