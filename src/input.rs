//! Module containing logic for storing what the user has typed in so far.
//!
//! Module containing logic for writing to the screen.

use std::ffi::{CStr, CString};

use wayland_client::EventQueueHandle;
use wayland_client::protocol::wl_keyboard;
use wayland_kbd::{self, ModifiersState, keysyms};

use pam::check_auth;

use libc::{getuid, getpwuid};

use ::color::Color;
use ::window::Window;

pub struct Input {
    /// ID for the window.
    window_id: usize,
    /// Buffer of what the user has input so far.
    buffer: String,
    /// Username, gotten from system during `new`.
    username: String,
    /// Boolean value saying if the user has logged in yet or not.
    logged_in: bool,
    /// Number of failed login attempts.
    failed: u32,
    /// The new color, if a new one needs to be generated.
    pub new_color: Option<Color>
}

impl Input {
    pub fn new(window_id: usize) -> Self {
        let username = unsafe {
            let uid = getuid();
            let pwuid = getpwuid(uid);
            let slice = CStr::from_ptr((*pwuid).pw_name);
            slice.to_string_lossy().into_owned()
        };
        Input {
            window_id,
            buffer: String::new(),
            username,
            logged_in: false,
            failed: 0,
            new_color: None
        }
    }

    /// Determines if the user has succesfully logged in yet.
    pub fn is_logged_in(&self) -> bool {
        self.logged_in
    }

    /// Update the color of the screen.
    pub fn update_screen_color(&mut self) {
        let (mut r, mut g, mut b) = (0u8, 0u8, 0u8);
        for chr in self.buffer.chars() {
            let val = unsafe {
                let val = chr as u32;
                let bytes: [u8; 4] = ::std::mem::transmute(val.to_be());
                bytes[3]
            };
            r = r.wrapping_add(val);
            b = b.wrapping_add(val / 2);
            g = g.wrapping_add(val / 3);
        }
        let color = Color::from_u8s(r, g, b);
        self.new_color = Some(color);

    }
}


impl wayland_kbd::Handler for Input {
    fn key(&mut self,
           _: &mut EventQueueHandle,
           _: &wl_keyboard::WlKeyboard,
           _: u32,
           _: u32,
           _: &ModifiersState,
           _: u32,
           keysym: u32,
           state: wl_keyboard::KeyState,
           text: Option<String>) {
        if let wl_keyboard::KeyState::Pressed = state {
            match keysym {
                keysyms::XKB_KEY_Return |
                keysyms::XKB_KEY_KP_Enter => {
                    let check_auth = unsafe {
                        let username = CString::new(self.username.as_str())
                            .expect("Username could not be C-String-ed");
                        let password = CString::new(self.buffer.as_str())
                            .expect("Password could not be C-String-ed");
                        check_auth(username.as_ptr(), password.as_ptr())
                    };
                    if check_auth {
                        self.logged_in = true;
                    } else {
                        self.failed += 1;
                        println!("Failed login attempt {}", self.failed);
                    }
                    self.buffer.clear()
                },
                keysyms::XKB_KEY_BackSpace => {
                    self.buffer.pop();
                }
                _ => {
                    if let Some(text) = text {
                        self.buffer.push_str(text.as_str());
                        self.update_screen_color();
                    }
                }
            }
        }
    }
}
