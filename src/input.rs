/// Module containing logic for storing what the user has typed in so far.

/// Module containing logic for writing to the screen.

use std::io::Write;
use wayland_client::EventQueueHandle;
use wayland_client::protocol::wl_keyboard;
use wayland_kbd::{self, ModifiersState, keysyms};

pub struct Input {
    buffer: String
}

impl Input {
    pub fn new() -> Self {
        Input {
            buffer: String::new()
        }
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
                    // TODO Submit this.
                    self.buffer.clear()
                },
                keysyms::XKB_KEY_BackSpace => {
                    self.buffer.pop();
                }
                _ => {
                    if let Some(text) = text {
                        self.buffer.push_str(text.as_str());
                    }
                }
            }
        }
    }
}
