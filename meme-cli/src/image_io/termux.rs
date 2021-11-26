use std::process::{Command, Stdio};

use anyhow::{anyhow, Error};
use image::{png::PngEncoder, RgbaImage};

pub fn image_out(img_buffer: &RgbaImage) -> Result<(), Error> {
    let mut child = Command::new("termux-share")
        .args(["-a", "send", "-c", "image/png"])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .spawn()?;
    let encoder = PngEncoder::new(child.stdin.take().unwrap());
    encoder.encode(
        &img_buffer,
        img_buffer.width(),
        img_buffer.height(),
        image::ColorType::Rgba8,
    )?;
    child
        .wait()?
        .success()
        .then(|| ())
        .ok_or(anyhow!("termux-send error!"))
}

pub fn image_in() -> Result<RgbaImage, Error> {
    Err(anyhow!("This isn't supported on termux."))
}
