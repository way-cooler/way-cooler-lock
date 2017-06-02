#[macro_use]
extern crate wayland_client;
extern crate tempfile;
extern crate byteorder;
extern crate wayland_kbd;
extern crate libc;
extern crate clap;

mod color;
mod input;
mod window;
mod pam;

use input::{Input};
use window::{Resolution, Window};

use clap::App;

use wayland_client::EnvHandler;
use wayland_client::protocol::{wl_compositor, wl_shell, wl_shm,
                               wl_seat, wl_keyboard,
                               wl_output, wl_registry};
use wayland_kbd::MappedKeyboard;

wayland_env!(WaylandEnv,
             compositor: wl_compositor::WlCompositor,
             seat: wl_seat::WlSeat,
             shell: wl_shell::WlShell,
             shm: wl_shm::WlShm,
             output: wl_output::WlOutput
);

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

    // Fetch the output now that it has been declared by the compositor.
    let output = get_output(env_id, registry, &mut event_queue);

    // Set up `Resolution`, which ensures the lockscreen is the same
    // size as the output, even if it resizes.
    let resolution = Resolution::new();
    let resolution_id = event_queue.add_handler(resolution);
    event_queue.register::<_, Resolution>(&output, resolution_id);

    // Dispatch so that the resolution is properly set in the handler.
    event_queue.dispatch().expect("Could not dispatch resolution");

    // Set up `Window`, which takes care of drawing to the buffer.
    // It uses the `Resolution` to determine how big to make the buffer.
    let window = Window::new(resolution_id, output, env_id, event_queue.state());
    let shell_surface = window.shell_surface();
    let window_id = event_queue.add_handler(window);
    event_queue.register::<_, Window>(&shell_surface, window_id);

    // Set up `Input`, which processes user input before passing it off to PAM
    // for authentication.
    let input = MappedKeyboard::new(Input::new()).ok()
        .expect("Could not create input handler");
    let input_id = event_queue.add_handler(input);
    let keyboard = get_keyboard(env_id, &mut event_queue);
    event_queue.register::<_, MappedKeyboard<Input>>(&keyboard, input_id);

    loop {
        display.flush()
            .expect("Could not flush display");
        event_queue.dispatch()
            .expect("Could not dispatch queue");
        let mut state = event_queue.state();
        let color = {
            let input = state.get_mut_handler::<MappedKeyboard<Input>>(input_id);
            if input.handler().is_logged_in() {
                break;
            }
            input.handler().new_color.take()
        };
        if let Some(color) = color {
            let res: Resolution = *state.get_handler(resolution_id);
            let window = state.get_mut_handler::<Window>(window_id);
            window.update_color(color, res);
        }
    }
}


fn get_output(env_id: usize,
              registry: wl_registry::WlRegistry,
              event_queue: &mut wayland_client::EventQueue)
              -> wl_output::WlOutput {
    let state = event_queue.state();
    let env = state.get_handler::<EnvHandler<WaylandEnv>>(env_id);
    for &(name, ref interface, version) in env.globals() {
        if interface == "wl_output" {
            return registry.bind::<wl_output::WlOutput>(version, name)
        }
    }
    for &(name, ref interface, version) in env.globals() {
        println!("{:4} : {} (version {})", name, interface, version);
    }
    panic!("Could not find wl_output to bind to");
}

fn get_keyboard(env_id: usize, event_queue: &mut wayland_client::EventQueue)
                -> wl_keyboard::WlKeyboard {
    let state = event_queue.state();
    let env = state.get_handler::<EnvHandler<WaylandEnv>>(env_id);
    env.seat.get_keyboard().expect("Seat was destroyed")
}
