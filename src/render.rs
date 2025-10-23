use parking_lot::{Condvar, Mutex};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use ul_next::View;
use ul_next::config::FontHinting;
use ul_next::{Config, Renderer, platform};

use crate::gpu::{OpenglCommandReceiver, create_gpu_driver};
use crate::{ArboardClipboard, LIB};

pub static mut UL_RENDERER: Option<&'static Renderer> = None;
pub static mut GL_RENDERER: Option<&'static OpenglCommandReceiver> = None;

pub static RENDER_MUTEX: Mutex<
    Option<
        Vec<(
            Box<
                dyn Fn(&mut HashMap<u32, View>, &mut HashMap<u32, Instant>, &mut bool) -> bool
                    + Send,
            >,
            Option<Arc<Condvar>>,
        )>,
    >,
> = Mutex::new(None);
pub static RENDER_COND: Condvar = Condvar::new();

pub fn renderer_pending(
    f: impl Fn(&mut HashMap<u32, View>, &mut HashMap<u32, Instant>, &mut bool) -> bool + Send + 'static,
) {
    let mut lock = RENDER_MUTEX.lock();
    let funcs = lock.as_mut().unwrap();
    funcs.push((Box::new(f), None));
}

pub fn renderer_run(
    f: impl Fn(&mut HashMap<u32, View>, &mut HashMap<u32, Instant>, &mut bool) -> bool + Send + 'static,
) {
    let c = Arc::new(Condvar::new());
    let mut lock = RENDER_MUTEX.lock();
    let funcs = lock.as_mut().unwrap();
    funcs.push((Box::new(f), Some(c.clone())));
    RENDER_COND.notify_one();
    c.wait(&mut lock);
}

static EXIT_RENDERER: AtomicBool = AtomicBool::new(false);

#[cfg(feature = "gl-headless")]
#[gl_headless::gl_headless]
pub fn renderer_main_wrapper() {
    renderer_main();
}

#[cfg(feature = "surfman")]
pub fn renderer_main_wrapper() {
    renderer_main();
}

#[allow(static_mut_refs)]
fn renderer_main() {
    #[cfg(feature = "surfman")]
    let (device, mut context) = {
        use surfman::{Connection, ContextAttributeFlags, ContextAttributes, GLVersion};

        let connection = Connection::new().expect("Failed to create connection");

        let adapter = connection
            .create_low_power_adapter()
            .expect("Failed to create adapter");
        let mut device = connection
            .create_device(&adapter)
            .expect("Failed to create device");

        let ctx_desc = device
            .create_context_descriptor(&ContextAttributes {
                version: GLVersion::new(4, 6),
                flags: ContextAttributeFlags::empty(),
            })
            .expect("Failed to create context descriptor");
        let mut context = device
            .create_context(&ctx_desc, None)
            .expect("Failed to create GL context");

        device
            .make_context_current(&mut context)
            .expect("Failed to make context current");

        gl::load_with(|s| device.get_proc_address(&context, s) as *const _);

        unsafe {
            let version = std::ffi::CStr::from_ptr(gl::GetString(gl::VERSION) as *const i8);
            println!("GL version: {}", version.to_string_lossy());
        }

        (device, context)
    };

    let lib = LIB.get().unwrap().clone();
    let (gpu_driver, mut gl_renderer) = create_gpu_driver();

    platform::enable_platform_fontloader(lib.clone());
    platform::enable_platform_filesystem(lib.clone(), "./ultralight").unwrap();
    platform::set_clipboard(lib.clone(), ArboardClipboard::new());
    platform::enable_default_logger(lib.clone(), "./ultralight/ultralight.log").unwrap();
    platform::set_gpu_driver(lib.clone(), gpu_driver);

    let config = Config::start()
        .font_hinting(FontHinting::Smooth)
        .build(lib)
        .unwrap();

    let ul_renderer = Renderer::create(config).unwrap();

    unsafe {
        UL_RENDERER
            .replace(std::mem::transmute(&ul_renderer))
            .map(|_| panic!("Renderer already initialized"));

        GL_RENDERER
            .replace(std::mem::transmute(&gl_renderer))
            .map(|_| panic!("GL Renderer already initialized"));
    }

    {
        let mut lock = RENDER_MUTEX.lock();
        *lock = Some(Vec::new());
        RENDER_COND.notify_one();
    }

    let mut next_frame_time = Instant::now();
    let mut wakeup_by_timeout = false;
    let mut views: HashMap<u32, View> = HashMap::new();
    let mut views_updated: HashMap<u32, Instant> = HashMap::new();
    while !EXIT_RENDERER.load(Ordering::SeqCst) {
        if wakeup_by_timeout {
            next_frame_time = Instant::now() + Duration::from_millis(16);

            for (id, view) in views.iter() {
                if let Some(updated) = views_updated.get(id) {
                    if updated.elapsed().as_millis() < 1000 {
                        view.set_needs_paint(true);
                    } else {
                        views_updated.remove(id);
                    }
                }
            }

            ul_renderer.update();
            ul_renderer.render();
            gl_renderer.render();
        }

        let mut lock = RENDER_MUTEX.lock();
        let funcs = lock.take().unwrap();
        let mut next_funcs = Vec::new();
        let mut force_redraw = false;
        for (f, c) in funcs {
            if f(&mut views, &mut views_updated, &mut force_redraw) {
                next_funcs.push((f, c.clone()));
            }
            if let Some(c) = c {
                c.notify_all();
                c.notify_one();
            }
        }
        lock.replace(next_funcs);

        if force_redraw {
            wakeup_by_timeout = true; // 触发重绘
            continue;
        }

        wakeup_by_timeout = RENDER_COND
            .wait_until(&mut lock, next_frame_time)
            .timed_out();
    }

    #[cfg(feature = "surfman")]
    device.destroy_context(&mut context).unwrap();
}
