use std::borrow::Cow;

use anyhow::{anyhow, Error};
use arboard::ImageData;
use image::RgbaImage;

pub fn image_out(img_buffer: RgbaImage) -> Result<(), Error> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_image(ImageData {
        width: img_buffer.width() as _,
        height: img_buffer.height() as _,
        bytes: Cow::Borrowed(&img_buffer),
    })?;

    Ok(())
}

pub fn image_in() -> Result<RgbaImage, Error> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.get_image().map_err(Error::from).and_then(|img| {
        image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned())
            .ok_or(anyhow!("image from clipboard not compatible"))
    })
}
