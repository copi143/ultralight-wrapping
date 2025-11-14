use std::rc::Rc;
use ul_next::{
    Library,
    app::App,
    javascript::{JSObject, JSPropertyAttributes, JSValue},
    platform,
    window::WindowFlags,
};

pub struct ModInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

fn main() {
    let lib = Library::linked();

    platform::enable_platform_filesystem(lib.clone(), "./examples").unwrap();

    let app = App::new(lib.clone(), None, None).unwrap();

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

    window.set_title("Javascript example");

    let overlay = window
        .create_overlay(window.width(), window.height(), 0, 0)
        .unwrap();

    {
        let ctx = overlay.view().lock_js_context();
        let global = ctx.global_object();

        let func = JSObject::new_function_with_callback(&ctx, |ctx, _this, _args| {
            println!(
                "Javascript returned {:?}",
                ctx.evaluate_script("JavascriptCallback();")
                    .unwrap()
                    .as_string()
                    .unwrap()
            );

            println!(
                "Javascript returned {:?}",
                ctx.global_object()
                    .get_property("JavascriptCallback")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .call_as_function(None, &[])
                    .unwrap()
                    .as_string()
                    .unwrap()
            );

            Ok(JSValue::new_string(ctx, "And Hello from Rust!<br>"))
        });

        global
            .set_property("GetRustMessage", &func, JSPropertyAttributes::default())
            .unwrap();

        let mods = JSObject::new(&ctx);

        global
            .set_property(
                "mods",
                &mods,
                JSPropertyAttributes {
                    read_only: true,
                    dont_enum: false,
                    dont_delete: true,
                },
            )
            .unwrap();
    }

    overlay.view().load_html("").unwrap();

    let app = Rc::new(app);
    let app_clone = app.clone();

    window.set_close_callback(move |_window| {
        app_clone.quit();
    });

    // run the main loop
    app.run();
}
