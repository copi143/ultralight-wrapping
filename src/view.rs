use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use ul_next::event::{KeyEvent, KeyEventCreationInfo, KeyEventModifiers, KeyEventType};
use ul_next::event::{MouseButton, MouseEvent, MouseEventType, ScrollEvent, ScrollEventType};
use ul_next::key_code::VirtualKeyCode;
use ul_next::view::ViewConfig;

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
        let Some(renderer) = (unsafe { UL_RENDERER }) else {
            panic!("UL Renderer not initialized");
        };

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

const GLFW_MOD_SHIFT: u32 = 0x0001;
const GLFW_MOD_CONTROL: u32 = 0x0002;
const GLFW_MOD_ALT: u32 = 0x0004;
const GLFW_MOD_SUPER: u32 = 0x0008;
const GLFW_MOD_CAPS_LOCK: u32 = 0x0010;
const GLFW_MOD_NUM_LOCK: u32 = 0x0020;

fn parse_glfw_modifiers(key_mods: u32) -> KeyEventModifiers {
    let _ = key_mods & GLFW_MOD_CAPS_LOCK;
    let _ = key_mods & GLFW_MOD_NUM_LOCK;
    KeyEventModifiers {
        alt: key_mods & GLFW_MOD_ALT != 0,
        ctrl: key_mods & GLFW_MOD_CONTROL != 0,
        meta: key_mods & GLFW_MOD_SUPER != 0,
        shift: key_mods & GLFW_MOD_SHIFT != 0,
    }
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_report_key_down(view_id: u32, scancode: u32, key_mods: u32) {
    renderer_pending(move |views, views_updated, _| {
        let lib = LIB.get().unwrap().clone();
        if let Some(view) = views.get(&view_id) {
            view.fire_key_event(
                KeyEvent::new(
                    lib,
                    KeyEventCreationInfo {
                        ty: KeyEventType::KeyDown,
                        modifiers: parse_glfw_modifiers(key_mods),
                        virtual_key_code: VirtualKeyCode::Unknown,
                        native_key_code: scancode as i32,
                        text: "",
                        unmodified_text: "",
                        is_keypad: false,
                        is_auto_repeat: false,
                        is_system_key: false,
                    },
                )
                .unwrap(),
            );
            views_updated.insert(view_id, Instant::now());
        }
        false
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_report_key_up(view_id: u32, scancode: u32, key_mods: u32) {
    renderer_pending(move |views, views_updated, _| {
        let lib = LIB.get().unwrap().clone();
        if let Some(view) = views.get(&view_id) {
            view.fire_key_event(
                KeyEvent::new(
                    lib,
                    KeyEventCreationInfo {
                        ty: KeyEventType::KeyUp,
                        modifiers: parse_glfw_modifiers(key_mods),
                        virtual_key_code: VirtualKeyCode::Unknown,
                        native_key_code: scancode as i32,
                        text: "",
                        unmodified_text: "",
                        is_keypad: false,
                        is_auto_repeat: false,
                        is_system_key: false,
                    },
                )
                .unwrap(),
            );
            views_updated.insert(view_id, Instant::now());
        }
        false
    });
}

#[unsafe(no_mangle)]
extern "C" fn ultralightui_report_input(view_id: u32, text: *const u8) {
    let text = read_c_string(text).to_string();
    renderer_pending(move |views, views_updated, _| {
        let lib = LIB.get().unwrap().clone();
        if let Some(view) = views.get(&view_id) {
            view.fire_key_event(
                KeyEvent::new(
                    lib,
                    KeyEventCreationInfo {
                        ty: KeyEventType::Char,
                        modifiers: KeyEventModifiers {
                            alt: false,
                            ctrl: false,
                            meta: false,
                            shift: false,
                        },
                        virtual_key_code: VirtualKeyCode::Unknown,
                        native_key_code: 0,
                        text: &text,
                        unmodified_text: &text,
                        is_keypad: false,
                        is_auto_repeat: false,
                        is_system_key: false,
                    },
                )
                .unwrap(),
            );
            views_updated.insert(view_id, Instant::now());
        }
        false
    });
}

#[unsafe(no_mangle)]
#[allow(static_mut_refs)]
extern "C" fn ultralightui_copy_from_view(view_id: u32, buf_ptr: *mut c_void, buf_size: usize) {
    let buf_ptr = buf_ptr as usize;
    renderer_run(move |views, _, _| {
        if let Some(view) = views.get(&view_id) {
            let target = view.render_target().unwrap();
            let Some(renderer) = (unsafe { GL_RENDERER }) else {
                panic!("GL Renderer not initialized");
            };
            let Some(tex) = renderer.get_texture_handle(target.texture_id) else {
                let buf_ptr = buf_ptr as *mut u8;
                for i in 0..buf_size {
                    unsafe { *buf_ptr.add(i) = 0 };
                }
                return false;
            };
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
