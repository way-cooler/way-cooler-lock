/// Module containing logic for writing to the screen.

use std::io::Write;
use std::os::unix::io::AsRawFd;

use wayland_client::{self, EventQueueHandle, EnvHandler, Proxy};
use wayland_client::protocol::{wl_shm, wl_shell_surface, wl_buffer, wl_output};
use byteorder::{NativeEndian, WriteBytesExt};
use tempfile;

use ::WaylandEnv;

/// Used to know how big to make the surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub w: u32,
    pub h: u32
}

impl Resolution {
    pub fn new() -> Self {
        Resolution {
            w: 0,
            h: 0
        }
    }

    /// Gets the size of the resolution (width * height)
    pub fn size(self) -> u32 {
        self.w * self.h
    }
}

/// The main window struct, containing the buffer backing the wayland surface,
/// and the file descriptor to the shared memory.
pub struct Window {
    /// ID for the `Resolution` struct
    resolution_id: usize,
    buffer: wl_buffer::WlBuffer,
    file: ::std::fs::File,
    shell_surface: wl_shell_surface::WlShellSurface
}

impl Window {
    // allocates a buffer to hold the surface data
    pub fn new(resolution_id: usize,
               env_id: usize,
               state: wayland_client::StateGuard) -> Self {
        let res: Resolution = *state.get_handler(resolution_id);
        assert_ne!(res.size(), 0, "Resolution was not properly initialized");
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
        let shell_surface = env.shell.get_shell_surface(&surface);
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

            Window {
                resolution_id,
                buffer,
                file,
                shell_surface
            }
    }

    pub fn shell_surface(&self) -> wl_shell_surface::WlShellSurface {
        self.shell_surface.clone()
            .expect("Shell surface was not initialized")
    }
}

impl wl_shell_surface::Handler for Window {
    fn ping(&mut self, _: &mut EventQueueHandle,
            me: &wl_shell_surface::WlShellSurface,
            serial: u32) {
        me.pong(serial);
    }

    // we ignore the other methods in this example, by default they do nothing
}

declare_handler!(Window,
                 wl_shell_surface::Handler,
                 wl_shell_surface::WlShellSurface);

impl wl_output::Handler for Resolution {
    fn mode(&mut self,
            _evqh: &mut EventQueueHandle,
            _proxy: &wl_output::WlOutput,
            _flags: wl_output::Mode,
            width: i32,
            height: i32,
            _refresh: i32) {
        self.w = width as u32;
        self.h = height as u32;
        println!("wxh: {}x{}", width, height);
    }
}

declare_handler!(Resolution, wl_output::Handler, wl_output::WlOutput);
