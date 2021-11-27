use std::process::Command;

use anyhow::{anyhow, Error};
use image::RgbaImage;

pub fn image_out(img_buffer: &RgbaImage) -> Result<(), Error> {
    let img_path = std::env::temp_dir().join("share.png");
    img_buffer.save(&img_path)?;
    Command::new("termux-share")
        .args(["-a", "send", img_path.to_str().unwrap()])
        .spawn()?
        .wait()?
        .success()
        .then(|| ())
        .ok_or(anyhow!("termux-send error!"))
}

pub fn image_in() -> Result<RgbaImage, Error> {
    Err(anyhow!("This isn't supported on termux."))
}
