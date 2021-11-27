use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Error};
use fontdue::{
    layout::{
        CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign,
        WrapStyle,
    },
    FontSettings,
};
use image::{save_buffer, DynamicImage, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

static FONT: &[u8] = include_bytes!("../resources/BebasNeue-Regular.ttf");

mod git_ops;

pub struct MemeTemplate {
    image: RgbaImage,
    config: MemeConfig,
}

impl MemeTemplate {
    pub fn render(
        mut self,
        text: &[String],
        _config: &Config,
        max_font_size: f32,
    ) -> Result<RgbaImage, Error> {
        let font = fontdue::Font::from_bytes(FONT, FontSettings::default()).unwrap();
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);

        let mut raster_cache = HashMap::new();

        for (text, bb) in text.iter().zip(&self.config.text) {
            let max_height = (bb.max.1 - bb.min.1) as f32;
            let max_width = (bb.max.0 - bb.min.0) as f32;
            let mut min = 5.;
            let mut max = max_font_size;

            let abs_max_lines = text.split(char::is_whitespace).count();
            let glyphs = loop {
                let candidate = min + max / 2.;
                layout.reset(&LayoutSettings {
                    max_height: Some(max_height),
                    max_width: Some(max_width),
                    horizontal_align: HorizontalAlign::Center,
                    vertical_align: VerticalAlign::Middle,
                    wrap_style: WrapStyle::Word,
                    ..Default::default()
                });
                layout.append(
                    &[&font],
                    &TextStyle {
                        text,
                        px: candidate,
                        font_index: 0,
                        user_data: (),
                    },
                );
                if layout.lines() > abs_max_lines
                    || layout.lines() > (max_height / candidate).ceil() as usize
                {
                    max = candidate;
                } else if min - max <= 0.25 {
                    break layout.glyphs();
                } else {
                    min = candidate;
                }
            };

            for glyph in glyphs {
                let (ref metrics, ref bytes) = raster_cache
                    .entry(glyph.key)
                    .or_insert_with(|| font.rasterize_config(glyph.key));

                for x in 0..metrics.width {
                    for y in 0..metrics.height {
                        let coverage = bytes[x + y * metrics.width] as f32 / u8::MAX as f32;
                        let x = bb.min.0 + x as u32 + glyph.x as u32;
                        let y = bb.min.1 + y as u32 + glyph.y as u32;
                        let existing_color = self.image.get_pixel(x, y);
                        let text_color = self.config.color.unwrap_or([0f32, 0., 0., 1.]);
                        let colors = existing_color.0;
                        let colors = [
                            ((text_color[0] * coverage
                                + (1. - coverage) * (colors[0] as f32 / u8::MAX as f32))
                                * u8::MAX as f32) as u8,
                            ((text_color[1] * coverage
                                + (1. - coverage) * (colors[1] as f32 / u8::MAX as f32))
                                * u8::MAX as f32) as u8,
                            ((text_color[2] * coverage
                                + (1. - coverage) * (colors[2] as f32 / u8::MAX as f32))
                                * u8::MAX as f32) as u8,
                            ((text_color[3] * coverage
                                + (1. - coverage) * (colors[3] as f32 / u8::MAX as f32))
                                * u8::MAX as f32) as u8,
                        ];

                        *self.image.get_pixel_mut(x, y) = Rgba(colors);
                    }
                }
            }
        }

        Ok(self.image)
    }
}

#[derive(Serialize, Deserialize)]
pub struct MemeConfig {
    pub color: Option<[f32; 4]>,
    pub text: Vec<MemeText>,
}

#[derive(Serialize, Deserialize)]
pub struct MemeText {
    pub min: (u32, u32),
    pub max: (u32, u32),
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    sources: Vec<MemeSource>,
}

impl Config {
    pub fn load() -> Result<Config, Error> {
        let config_path = dirs::config_dir()
            .ok_or_else(|| anyhow!("config dir not found"))?
            .join("memecli.conf.json");
        match fs::read_to_string(config_path) {
            Ok(config_str) => Ok(serde_json::from_str(&config_str)?),
            Err(_) => Ok(Config {
                sources: vec![MemeSource::GitUrl {
                    url: "https://github.com/TheRawMeatball/memeinator-memesrc.git".to_owned(),
                    alias: "default".to_owned(),
                }],
            }),
        }
    }

    pub fn fetch_source_list(&self) -> impl Iterator<Item = &MemeSource> + '_ {
        self.sources.iter()
    }
    pub fn fetch_template_list(&self) -> impl Iterator<Item = String> + '_ {
        self.fetch_source_list()
            .flat_map(MemeSource::to_path)
            .flat_map(|path| path.read_dir())
            .flatten()
            .flatten()
            .flat_map(|meme_dir| meme_dir.file_name().into_string())
    }

    pub fn get_meme_template(&self, template: &str) -> Result<MemeTemplate, Error> {
        for source in &self.sources {
            let source_dir = source.to_path()?;

            for meme_dir in source_dir.read_dir().with_context(|| {
                format!(
                    "Cannot open meme source folder {}",
                    source_dir.to_str().unwrap()
                )
            })? {
                let meme_dir = meme_dir?;
                if !meme_dir.metadata()?.is_dir() {
                    continue;
                }
                let template_name = meme_dir
                    .file_name()
                    .into_string()
                    .map_err(|_| anyhow!("can't parse template name as utf8"))?;

                if template_name == template {
                    let dir_path = meme_dir.path();
                    let config_path = dir_path.join("config.json");

                    let config = serde_json::from_str(&fs::read_to_string(config_path)?)?;

                    let img = image::png::PngDecoder::new(
                        fs::File::open(dir_path.join("image.png")).with_context(|| {
                            format!("Cannot read image.png for format {}", &template_name)
                        })?,
                    )?;
                    let image = DynamicImage::from_decoder(img)?.to_rgba8();

                    return Ok(MemeTemplate { config, image });
                }
            }
        }

        Err(anyhow!("Can't find template {}", template))
    }

    /// Writes a template to the first local meme source
    pub fn write_template(
        &self,
        buf: &[u8],
        width: u32,
        height: u32,
        config: MemeConfig,
        name: &str,
    ) -> Result<(), Error> {
        let source_path = self
            .sources
            .iter()
            .find_map(|source| match source {
                MemeSource::GitUrl { .. } => None,
                MemeSource::LocalPath(path) => Some(Path::new(path)),
            })
            .ok_or(anyhow!("No local sources configured"))?;

        let meme_path = source_path.join(name);
        std::fs::create_dir(&meme_path)?;
        save_buffer(
            meme_path.join("image.png"),
            buf,
            width,
            height,
            image::ColorType::Rgba8,
        )?;
        let config = serde_json::to_string_pretty(&config)?;
        fs::write(meme_path.join("config.json"), config.as_bytes())?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub enum MemeSource {
    GitUrl { url: String, alias: String },
    LocalPath(String),
}

impl MemeSource {
    pub fn to_path_and_update(&self) -> Result<PathBuf, Error> {
        let cache = dirs::cache_dir()
            .ok_or_else(|| anyhow!("cache dir not found"))?
            .join("memecli");
        let path = match self {
            MemeSource::GitUrl { url, alias } => {
                let path = cache.join(&alias);
                if path.is_dir() {
                    println!("Updating meme repository {} ({})", alias, url);
                    git_ops::update_repo(&path)?;
                } else {
                    println!("Cloning meme repository {} ({})", alias, url);
                    fs::create_dir_all(&path)?;
                    git_ops::clone_repo(&path, url)?;
                }
                path
            }
            MemeSource::LocalPath(path) => PathBuf::from(path),
        };
        Ok(path)
    }

    pub fn to_path(&self) -> Result<PathBuf, Error> {
        let cache = dirs::cache_dir().unwrap().join("memecli");
        let source = match self {
            MemeSource::GitUrl { alias, .. } => cache.join(alias),
            MemeSource::LocalPath(path) => PathBuf::from(path),
        };
        fs::create_dir_all(&source)?;
        Ok(source)
    }
}
