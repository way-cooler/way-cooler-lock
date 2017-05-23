#[macro_use]
extern crate wayland_client;

use wayland_client::{EventQueueHandle, EnvHandler, Proxy};
use wayland_client::protocol::{wl_compositor, wl_shell, wl_shm, wl_shell_surface,
                               wl_seat, wl_keyboard, wl_surface, wl_output};

wayland_env!(WaylandEnv,
             //compositor: wl_compositor::WlCompositor,
             //seat: wl_seat::WlSeat,
             //shell: wl_shell::WlShell,
             //shm: wl_shm::WlShm,
             output: wl_output::WlOutput
);


struct LockScreen {
    
}

impl wl_output::Handler for LockScreen {
    fn mode(&mut self,
            evqh: &mut EventQueueHandle,
            proxy: &wl_output::WlOutput,
            flags: wl_output::Mode,
            width: i32,
            height: i32,
            refresh: i32) {
        println!("wxh: {}x{}", width, height);
    }
}

declare_handler!(LockScreen, wl_output::Handler, wl_output::WlOutput);

fn main() {
    let (display, mut event_queue) = match wayland_client::default_connect() {
        Ok(ret) => ret,
        Err(e) => panic!("Cannot connect to wayland server: {:?}", e)
    };
    let env_id = event_queue.add_handler(EnvHandler::<WaylandEnv>::new());
    let registry = display.get_registry();
    event_queue.register::<_, EnvHandler<WaylandEnv>>(&registry, env_id);
    // a roundtrip sync will dispatch all event declaring globals to the handler
    event_queue.sync_roundtrip().unwrap();

    // Can now fetch the output and associate the lock screen

    let output;
    {
        let state = event_queue.state();
        let env = state.get_handler::<EnvHandler<WaylandEnv>>(0);
        // Have to bind a new one, cause yah do
        output = registry.bind::<wl_output::WlOutput>(2, 9);
    }

    let lock_screen = LockScreen {};
    let env_id = event_queue.add_handler(lock_screen);
    event_queue.register::<_, LockScreen>(&output, env_id);

    event_queue.dispatch().unwrap();
}
