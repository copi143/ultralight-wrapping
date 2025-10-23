// use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use ul_next::event::{MouseButton, MouseEvent, MouseEventType, ScrollEvent, ScrollEventType};
use ul_next::view::ViewConfig;

// pub struct Recipe {
//     pub name: String,
//     pub material: BTreeMap<String, u64>,
//     pub products: BTreeMap<String, u64>,
//     pub timecost: u64,
// }

use crate::render::{GL_RENDERER, UL_RENDERER, renderer_pending, renderer_run};
use crate::{LIB, read_c_string};

#[unsafe(no_mangle)]
#[allow(static_mut_refs)]
extern "C" fn ultralightui_create_view(
    url: *const u8,
    width: u32,
    height: u32,
    transparent: u32,
) -> u32 {
    let url = read_c_string(url).to_string();

    let id = Arc::new(AtomicU32::new(0));

    let id_clone = id.clone();
    renderer_run(move |views, views_updated, _| {
        let lib = LIB.get().unwrap().clone();
        let renderer = unsafe { UL_RENDERER.as_ref().unwrap() };

        let view_config = ViewConfig::start()
            .is_accelerated(true)
            .is_transparent(transparent != 0)
            .build(lib.clone())
            .unwrap();

        let view = renderer
            .create_view(width, height, &view_config, None)
            .unwrap();

        view.load_url(&url).unwrap();

        let id = views.len() as u32;
        id_clone.store(id, Ordering::SeqCst);
        views.insert(id, view);
        views_updated.insert(id, Instant::now());

        false
    });

    id.load(Ordering::SeqCst)
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_view_set_size(view_id: u32, width: u32, height: u32) {
    renderer_run(move |views, views_updated, force_redraw| {
        if let Some(view) = views.get(&view_id) {
            view.resize(width, height);
            views_updated.insert(view_id, Instant::now());
        }
        *force_redraw = true;
        false
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_remove_view(view_id: u32) {
    renderer_run(move |views, _, _| {
        views.remove(&view_id);
        false
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_report_mouse_move(view_id: u32, x: i32, y: i32) {
    renderer_pending(move |views, views_updated, _| {
        let lib = LIB.get().unwrap().clone();
        if let Some(view) = views.get(&view_id) {
            view.fire_mouse_event(
                MouseEvent::new(lib, MouseEventType::MouseMoved, x, y, MouseButton::None).unwrap(),
            );
            views_updated.insert(view_id, Instant::now());
        }
        false
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_report_mouse_down(view_id: u32, x: i32, y: i32, button: u32) {
    renderer_pending(move |views, views_updated, _| {
        let lib = LIB.get().unwrap().clone();
        if let Some(view) = views.get(&view_id) {
            let mouse_button = match button {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                _ => MouseButton::None,
            };
            view.fire_mouse_event(
                MouseEvent::new(lib, MouseEventType::MouseDown, x, y, mouse_button).unwrap(),
            );
            views_updated.insert(view_id, Instant::now());
        }
        false
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_report_mouse_up(view_id: u32, x: i32, y: i32, button: u32) {
    renderer_pending(move |views, views_updated, _| {
        let lib = LIB.get().unwrap().clone();
        if let Some(view) = views.get(&view_id) {
            let mouse_button = match button {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                _ => MouseButton::None,
            };
            view.fire_mouse_event(
                MouseEvent::new(lib, MouseEventType::MouseUp, x, y, mouse_button).unwrap(),
            );
            views_updated.insert(view_id, Instant::now());
        }
        false
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_report_scroll(view_id: u32, x: i32, y: i32) {
    renderer_pending(move |views, views_updated, _| {
        let lib = LIB.get().unwrap().clone();
        if let Some(view) = views.get(&view_id) {
            view.fire_scroll_event(
                ScrollEvent::new(lib, ScrollEventType::ScrollByPixel, x, y).unwrap(),
            );
            views_updated.insert(view_id, Instant::now());
        }
        false
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_report_focus(view_id: u32, focused: u32) {
    renderer_pending(move |views, views_updated, _| {
        if let Some(view) = views.get(&view_id) {
            if focused != 0 {
                view.focus();
            } else {
                view.unfocus();
            }
            views_updated.insert(view_id, Instant::now());
        }
        false
    });
}

// #[unsafe(no_mangle)]
// extern "C" fn ultralightui_report_key_down(view_id: u32, key_code: u32, key_char: u16) {
//     renderer_pending(move |views, _, _| {
//         let lib = LIB.get().unwrap().clone();
//         if let Some(view) = views.get(&view_id) {
//             view.fire_key_event(
//                 KeyEvent::new(
//                     lib,
//                     KeyEventCreationInfo {
//                         ty: KeyEventType::KeyDown,
//                         virtual_key_code: key_code,
//                         native_key_code: 0,
//                         text: String::from_utf16_lossy(&[key_char]),
//                         unmodified_text: String::from_utf16_lossy(&[key_char]),
//                         is_keypad: false,
//                         is_auto_repeat: false,
//                         modifiers: 0,
//                     },
//                 )
//                 .unwrap(),
//             );
//         }
//         false
//     });
// }

#[unsafe(no_mangle)]
#[allow(static_mut_refs)]
extern "C" fn ultralightui_copy_from_view(view_id: u32, buf_ptr: *mut c_void, buf_size: usize) {
    let buf_ptr = buf_ptr as usize;
    renderer_run(move |views, _, _| {
        if let Some(view) = views.get(&view_id) {
            let target = view.render_target().unwrap();
            let renderer = unsafe { GL_RENDERER.as_ref().unwrap() };
            let tex = renderer.get_texture_handle(target.texture_id).unwrap();
            let width = target.width as usize;
            let height = target.height as usize;
            if buf_size < width * height * 4 {
                eprintln!(
                    "Buffer size too small: got {}, need {}",
                    buf_size,
                    width * height * 4
                );
                return false;
            }
            if buf_size > width * height * 4 {
                eprintln!(
                    "Warning: buffer size larger than needed: got {}, need {}",
                    buf_size,
                    width * height * 4
                );
            }

            unsafe {
                gl::PixelStorei(gl::PACK_ALIGNMENT, 4);
                gl::PixelStorei(gl::PACK_ROW_LENGTH, width as i32);
                gl::PixelStorei(gl::PACK_SKIP_ROWS, 0);
                gl::PixelStorei(gl::PACK_SKIP_PIXELS, 0);
                gl::PixelStorei(gl::PACK_SWAP_BYTES, 0);
                gl::PixelStorei(gl::PACK_LSB_FIRST, 0);
                gl::BindTexture(gl::TEXTURE_2D, tex);
                gl::GetTexImage(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    buf_ptr as *mut c_void,
                );
                gl::BindTexture(gl::TEXTURE_2D, 0);
            }

            let err = unsafe { gl::GetError() };
            if err != gl::NO_ERROR {
                eprintln!("glGetTexImage error: {}", err);
            }
        }
        false
    });
}
