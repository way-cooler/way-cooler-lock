#[macro_use]
extern crate wayland_client;
extern crate tempfile;
extern crate byteorder;

use std::io::Write;
use std::os::unix::io::AsRawFd;

use wayland_client::{EventQueueHandle, EnvHandler, Proxy};
use wayland_client::protocol::{wl_compositor, wl_shell, wl_shm, wl_shell_surface, wl_buffer,
                               wl_seat, wl_keyboard, wl_surface, wl_output};
use byteorder::{NativeEndian, WriteBytesExt};


wayland_env!(WaylandEnv,
             compositor: wl_compositor::WlCompositor,
             //seat: wl_seat::WlSeat,
             shell: wl_shell::WlShell,
             shm: wl_shm::WlShm,
             output: wl_output::WlOutput
);

/// Used to know how big to make the surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Resolution {
    w: u32,
    h: u32
}

impl Resolution {
    fn size(self) -> u32 {
        self.w * self.h
    }
}

struct Window {
    buffer: wl_buffer::WlBuffer,
    file: std::fs::File,
}

struct LockScreen {
    /// Optional resolution, filled in when wl_output::mode is triggered.
    pub res: Option<Resolution>,
    pub window: Option<Window>
}


impl LockScreen {
    fn new() -> Self {
        LockScreen {
            res: None,
            window: None
        }
    }

    // allocates a buffer to hold the surface data
    fn attach_buffer(&mut self, win: Window) {
        self.window = Some(win);
    }
}

impl wl_shell_surface::Handler for LockScreen {
    fn ping(&mut self, _: &mut EventQueueHandle, me: &wl_shell_surface::WlShellSurface, serial: u32) {
        me.pong(serial);
    }

    // we ignore the other methods in this example, by default they do nothing
}

declare_handler!(LockScreen,
                 wl_shell_surface::Handler,
                 wl_shell_surface::WlShellSurface);


impl wl_output::Handler for LockScreen {
    fn mode(&mut self,
            evqh: &mut EventQueueHandle,
            proxy: &wl_output::WlOutput,
            flags: wl_output::Mode,
            width: i32,
            height: i32,
            refresh: i32) {
        self.res = Some(Resolution {
            w: width as u32,
            h: height as u32
        });
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
        let env = state.get_handler::<EnvHandler<WaylandEnv>>(env_id);
        // Have to bind a new one, cause yah do
        output = registry.bind::<wl_output::WlOutput>(2, 9);
    }

    let mut lock_screen = LockScreen::new();
    let lock_screen_id = event_queue.add_handler(lock_screen);
    event_queue.register::<_, LockScreen>(&output, lock_screen_id);

    event_queue.dispatch().unwrap();
    let shell_surface;
    {
        // get the buffer size out, allocate it, drop the env
        // and then pass to lock_screen with a new mutable borrow
        let mut state = event_queue.state();
        // Get the resolution
        let res = {
            let lock_screen: &LockScreen = state.get_handler(lock_screen_id);
            lock_screen.res.clone().unwrap()
        };

        let (buffer, file) = {
            let env = state.get_handler::<EnvHandler<WaylandEnv>>(env_id);
            // Create buffer, write bytes into buffer
            let mut file = tempfile::tempfile().ok()
                .expect("Unable to create buffer file");
            for _ in 0..(res.size()) {
                file.write_u32::<NativeEndian>(0).unwrap();
            }
            file.flush().unwrap();
            // Create surface
            let surface = env.compositor.create_surface();
            shell_surface = env.shell.get_shell_surface(&surface);
            let pool = env.shm.
                create_pool(file.as_raw_fd(), (res.w * res.h * 4) as i32);
            let buffer = pool.create_buffer(0,
                                            res.w as i32,
                                            res.h as i32,
                                            (res.w * 4) as i32,
                                            wl_shm::Format::Argb8888)
                .expect("Pool is already dead");
            shell_surface.set_toplevel();
            surface.attach(Some(&buffer), 0, 0);
            surface.commit();
            (buffer, file)
        };
        // Now attach buffer
        {
            let lock_screen: &mut LockScreen = state.get_mut_handler(lock_screen_id);
            lock_screen.attach_buffer(Window { buffer, file});
        }
    }
    // Register this supporting the shell interface
    event_queue.register::<_, LockScreen>(&shell_surface, lock_screen_id);
    loop {
        display.flush().unwrap();
        event_queue.dispatch().unwrap();
    }
}
