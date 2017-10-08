//! Takes the current screen and blurs it so that you can't see what
//! is going on.
use dbus::{Connection, Message, BusType};
use dbus::arg::Array;

use wayland_client;
use ::window::{Window, Resolution};
use image::png::PNGEncoder;
use image::{ColorType, DynamicImage, load_from_memory};

/// How long to wait until d-bus timeout
const DBUS_WAIT_TIME: i32 = 10000;

// Bit depth of image.
const BIT_DEPTH: u8 = 8;

pub struct Blur {
    pub window_id: usize,
    pub image: DynamicImage
}

impl Blur {
    pub fn new(resolution_id: usize,
               window_id: usize,
               mut state: wayland_client::StateGuard)
               -> Self {
        let res: Resolution = *state.get_handler(resolution_id);
        let window: &mut Window = state.get_mut_handler(window_id);
        let image = get_screen(res);
        window.write_bytes(res, &image.to_rgba().into_raw());
        Blur {
            window_id,
            image
        }
    }


    pub fn blur(&mut self,
                amount: f32,
                resolution_id: usize,
                mut state: wayland_client::StateGuard) {
        // TODO FIXME This is a bottle neck :(
        // See this issue https://github.com/PistonDevelopers/image/issues/615
        self.image = self.image.blur(amount);
        let res: Resolution = *state.get_handler(resolution_id);
        let window: &mut Window = state.get_mut_handler(self.window_id);
        window.write_bytes(res, &self.image.to_rgba().into_raw());
    }
}

fn get_screen(res: Resolution) -> DynamicImage {
    let con = Connection::get_private(BusType::Session)
        .expect("Could not get d-bus connection");
    let screen_msg = Message::new_method_call("org.way-cooler",
                                              "/org/way_cooler/Screen",
                                              "org.way_cooler.Screen",
                                              "Scrape")
        .expect("Could not construct message -- is Way Cooler running?");
    let reply = con.send_with_reply_and_block(screen_msg, DBUS_WAIT_TIME)
        .expect("Could not talk to Way Cooler -- is Way Cooler running?");
    let mut pixels = reply.get1::<Array<u8, _>>()
        .expect("Way Cooler returned an unexpected value")
        .collect::<Vec<u8>>();
    convert_to_png(&mut pixels);
    let mut png_buf = Vec::with_capacity(4 * (res.w * res.h) as usize);
    {
        let encoder = PNGEncoder::new(&mut png_buf);
        encoder.encode(pixels.as_slice(), res.w, res.h, ColorType::RGBA(BIT_DEPTH))
            .expect("Could not encode image to PNG");
    }
    let mut image = load_from_memory(png_buf.as_slice())
        .expect("Could not read encoded image");
    image = image.flipv();
    let mut image_rgba = image.to_rgba();
    // TODO Split this into its own function
    {
        let pixels = image_rgba.enumerate_pixels_mut();
        for (_x, _y, pixel) in pixels {
            let alpha = pixel[3] as u32;
            pixel[0] = rgba_conversion(pixel[0], alpha);
            pixel[1] = rgba_conversion(pixel[1], alpha);
            pixel[2] = rgba_conversion(pixel[2], alpha);

            let tmp = pixel[2];
            pixel[2] = pixel[0];
            pixel[0] = tmp;
        }
    }
    DynamicImage::ImageRgba8(image_rgba)
}

fn convert_to_png(buffer: &mut Vec<u8>) {
    let mut length = buffer.len();
    length -= length % 4;
    let mut i = 0;
    while i < length {
        // a b c d -> d a b c
        buffer[i + 2] ^= buffer[i + 3];
        buffer[i + 3] = buffer[i + 2] ^ buffer[i + 3];
        buffer[i + 2] ^= buffer[i + 3];
        buffer[i] ^= buffer[i + 2];
        buffer[i + 2] = buffer[i] ^ buffer[i + 2];
        buffer[i] ^= buffer[i + 2];
        buffer[i + 2] ^= buffer[i + 1];
        buffer[i + 1] = buffer[i + 1] ^ buffer[i + 2];
        buffer[i + 2] ^= buffer[i + 1];
        i += 4;
    }
}

fn rgba_conversion(num: u8, third_num: u32) -> u8 {
    let big_num = num as u32;
    ((big_num * third_num) / 255) as u8
}
