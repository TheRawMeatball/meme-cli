use std::borrow::Cow;

use anyhow::{anyhow, Error};
use arboard::ImageData;
use image::RgbaImage;
use memeinator::RenderedMeme;

pub fn image_out(rendered: RenderedMeme) -> Result<(), Error> {
    match rendered {
        RenderedMeme::Simple(img_buffer) => {
            let mut clipboard = arboard::Clipboard::new()?;
            clipboard.set_image(ImageData {
                width: img_buffer.width() as _,
                height: img_buffer.height() as _,
                bytes: Cow::Borrowed(&img_buffer),
            })?
        }
        RenderedMeme::Animated(_) => return Err(anyhow!("Animated memes are not yet supported for clipboard export :( Please export to a file using -o")),
    };
    Ok(())
}

pub fn image_in() -> Result<RgbaImage, Error> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.get_image().map_err(Error::from).and_then(|img| {
        image::RgbaImage::from_raw(img.width as u32, img.height as u32, img.bytes.into_owned())
            .ok_or(anyhow!("image from clipboard not compatible"))
    })
}
