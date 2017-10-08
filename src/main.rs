#[macro_use] extern crate wayland_client;
#[macro_use] extern crate wayland_sys;
extern crate tempfile;
extern crate byteorder;
extern crate wayland_kbd;
extern crate libc;
extern crate clap;
extern crate dbus;
extern crate image;

mod color;
mod input;
mod window;
mod pam;
mod effects;
use effects::Blur;

use input::{Input};
use window::{Resolution, Window};

use clap::App;

use wayland_client::EnvHandler;
use wayland_client::protocol::{wl_compositor, wl_shell, wl_shm,
                               wl_seat, wl_keyboard, wl_output};
use wayland_kbd::MappedKeyboard;
use wl_compositor::WlCompositor;

wayland_env!(WaylandEnv,
             compositor: wl_compositor::WlCompositor,
             seat: wl_seat::WlSeat,
             shell: wl_shell::WlShell,
             shm: wl_shm::WlShm,
             output: wl_output::WlOutput
);

// TODO Library
#[macro_export]
macro_rules! get_wayland {
    ($env_id: tt, $registry: expr, $event_queue: expr, $type: ty, $name: tt) => {{
        let state = $event_queue.state();
        let env = state.get_handler::<EnvHandler<WaylandEnv>>($env_id);
        let mut value = None;
        for &(name, ref interface, version) in env.globals() {
            if interface == $name {
                value = Some($registry.bind::<$type>(version, name));
                break;
            }
        }
        match value {
            None => {
                for &(name, ref interface, version) in env.globals() {
                    eprintln!("{:4} : {} (version {})", name, interface, version);
                }
                eprintln!(concat!("Could not find the ", $name, " protocol!"));
                None
            },
            v => v
        }
    }}
}
#[macro_export]
macro_rules! get_all_wayland {
    ($env_id: tt, $registry: expr, $event_queue: expr, $type: ty, $name: tt) => {{
        let state = $event_queue.state();
        let env = state.get_handler::<EnvHandler<WaylandEnv>>($env_id);
        let mut value = None;
        for &(name, ref interface, version) in env.globals() {
            if interface == $name {
                let mut list = value.take().unwrap_or_else(Vec::new);
                list.push($registry.bind::<$type>(version, name));
                value = Some(list);
            }
        }
        match value {
            None => {
                for &(name, ref interface, version) in env.globals() {
                    eprintln!("{:4} : {} (version {})", name, interface, version);
                }
                eprintln!(concat!("Could not find the ", $name, " protocol!"));
                None
            },
            v => v
        }
    }}
}

mod generated {
    // Generated code generally doesn't follow standards
    #![allow(dead_code,non_camel_case_types,unused_unsafe,unused_variables)]
    #![allow(non_upper_case_globals,non_snake_case,unused_imports)]

    pub mod interfaces {
        #[doc(hidden)]
        use wayland_client::protocol_interfaces::{wl_output_interface, wl_surface_interface};
        include!(concat!(env!("OUT_DIR"), "/desktop-shell_interface.rs"));
    }

    pub mod client {
        #[doc(hidden)]
        use wayland_client::{Handler, Liveness, EventQueueHandle, Proxy, RequestResult};
        #[doc(hidden)]
        use wayland_client::protocol::{wl_compositor, wl_shell, wl_shm, wl_surface,
                                       wl_seat, wl_keyboard, wl_buffer,
                                       wl_output, wl_registry};
        use super::interfaces;
        include!(concat!(env!("OUT_DIR"), "/desktop-shell_api.rs"));
    }
}

use generated::client::desktop_shell::DesktopShell;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let _matches = App::new("wc-lock")
        .version(VERSION)
        .author("Timidger <APragmaticPlace@gmail.com>")
        .about("Lock screen for Way Cooler window manager")
        .get_matches();

    let (display, mut event_queue) = match wayland_client::default_connect() {
        Ok(ret) => ret,
        Err(e) => panic!("Cannot connect to wayland server: {:?}", e)
    };
    // Associate the main environment handler to event queue.
    let env_id = event_queue.add_handler(EnvHandler::<WaylandEnv>::new());
    let registry = display.get_registry();
    event_queue.register::<_, EnvHandler<WaylandEnv>>(&registry, env_id);
    // a roundtrip sync will dispatch all event declaring globals to the handler
    // This will make all the globals usable.
    event_queue.sync_roundtrip().expect("Could not sync roundtrip");
    let compositor = get_wayland!(env_id, &registry, &mut event_queue, WlCompositor, "wl_compositor").unwrap();

    let desktop_shell = match get_wayland!(env_id, &registry, &mut event_queue, DesktopShell, "desktop_shell") {
        Some(shell) => shell,
        None => {
            eprintln!("Please make sure you're running the correct version of Way Cooler");
            eprintln!("This program only supports versions >= 0.7");
            ::std::process::exit(1);
        }
    };
    // Fetch the output now that it has been declared by the compositor.
    use wl_output::WlOutput;
    let outputs = get_all_wayland!(env_id, registry, &mut event_queue, WlOutput, "wl_output")
        .expect("Could not get outputs");
    let resolutions: Vec<usize> = outputs.iter()
        .map(|output| {
            let res = Resolution::new();
            let resolution_id = event_queue.add_handler(res);
            event_queue.register::<_, Resolution>(output, resolution_id);
            resolution_id
        }).collect();
    let mut blurs = Vec::with_capacity(outputs.len());
    let mut windows = Vec::with_capacity(outputs.len());
    // Set up `Input`, which processes user input before passing it off to PAM
    // for authentication.
    let input = MappedKeyboard::new(Input::new()).ok()
        .expect("Could not create input handler");
    let input_id = event_queue.add_handler(input);
    let keyboard = get_keyboard(env_id, &mut event_queue);
    event_queue.register::<_, MappedKeyboard<Input>>(&keyboard, input_id);
    event_queue.dispatch().expect("Could not dispatch resolution");
    for (output, resolution_id) in outputs.iter().zip(resolutions.clone()) {
        // Set up `Resolution`, which ensures the lockscreen is the same
        // size as the output, even if it resizes.
        event_queue.register::<_, Resolution>(&output, resolution_id);
        let surface = compositor.create_surface();

        desktop_shell.set_lock_surface(&output, &surface);
        event_queue.dispatch_pending().unwrap();
        // Set up `Window`, which takes care of drawing to the buffer.
        // It uses the `Resolution` to determine how big to make the buffer.
        let window = Window::new(resolution_id, surface, output, env_id, event_queue.state());
        let shell_surface = window.shell_surface();
        let window_id = event_queue.add_handler(window);
        event_queue.register::<_, Window>(&shell_surface, window_id);
        let blur = Blur::new(resolution_id, window_id, event_queue.state());
        windows.push(window_id);

        blurs.push(blur);
    }

    // TODO parametrize
    let mut blur_times = 6;
    let blur_amount = 1.0;
    event_queue.dispatch()
        .expect("Could not dispatch queue");
    'main: loop {
        display.flush()
            .expect("Could not flush display");
        event_queue.dispatch_pending()
            .expect("Could not dispatch queue");
        if blur_times >= 0 {
            for (blur, resolution_id) in blurs.iter_mut().zip(resolutions.clone()) {
                blur.blur(blur_amount, resolution_id, event_queue.state());
                blur_times -= 1;
            }
            continue;
        }
        event_queue.dispatch()
            .expect("Could not dispatch queue");
        let mut state = event_queue.state();
        let zipped = resolutions.clone().into_iter()
            .zip(windows.clone());
        let color = {
            let input = state.get_mut_handler::<MappedKeyboard<Input>>(input_id);
            let handler = input.handler();
            if handler.is_logged_in() {
                desktop_shell.unlock();
                break 'main;
            }
            handler.new_color.take()
        };
        for (resolution_id, window_id) in zipped {
            if let Some(color) = color {
                let res: Resolution = *state.get_handler(resolution_id);
                let window = state.get_mut_handler::<Window>(window_id);
                window.update_color(color, res);
            }
        }
    }
    event_queue.dispatch()
        .expect("Could not dispatch queue");
}

fn get_keyboard(env_id: usize, event_queue: &mut wayland_client::EventQueue)
                -> wl_keyboard::WlKeyboard {
    let state = event_queue.state();
    let env = state.get_handler::<EnvHandler<WaylandEnv>>(env_id);
    env.seat.get_keyboard().expect("Seat was destroyed")
}
