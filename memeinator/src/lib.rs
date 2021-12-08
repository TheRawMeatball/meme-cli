use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Error};
use fontdue::{
    layout::{
        CoordinateSystem, GlyphPosition, GlyphRasterConfig, HorizontalAlign, Layout,
        LayoutSettings, TextStyle, VerticalAlign, WrapStyle,
    },
    Font, FontSettings, Metrics,
};
use image::{save_buffer, AnimationDecoder, DynamicImage, Frame, GrayImage, Luma, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

static FONT: &[u8] = include_bytes!("../resources/BebasNeue-Regular.ttf");

mod git_ops;

pub enum MemeTemplate {
    Simple(SimpleMemeTemplate),
    Animated(AnimatedMemeTemplate),
}

pub enum RenderedMeme {
    Simple(RgbaImage),
    Animated(Vec<Frame>),
}

impl MemeTemplate {
    pub fn render(
        self,
        text: &[String],
        config: &Config,
        max_font_size: f32,
        watermark_msg: Option<&str>,
    ) -> RenderedMeme {
        match self {
            MemeTemplate::Simple(simple) => {
                RenderedMeme::Simple(simple.render(text, config, max_font_size, watermark_msg))
            }
            MemeTemplate::Animated(animated) => {
                RenderedMeme::Animated(animated.render(text, config, max_font_size, watermark_msg))
            }
        }
    }
}

pub struct SimpleMemeTemplate {
    image: RgbaImage,
    config: MemeConfig,
}

pub struct AnimatedMemeTemplate {
    frames: Vec<Frame>,
    config: MemeConfig,
}

impl SimpleMemeTemplate {
    pub fn render(
        mut self,
        text: &[String],
        config: &Config,
        max_font_size: f32,
        watermark_msg: Option<&str>,
    ) -> RgbaImage {
        let font = fontdue::Font::from_bytes(FONT, FontSettings::default()).unwrap();
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);

        let mut raster_cache = HashMap::new();

        for (text, bb) in text.iter().zip(&self.config.text) {
            let max_height = bb.max.1 - bb.min.1;
            let max_width = bb.max.0 - bb.min.0;
            let mask = render_text(
                &mut raster_cache,
                &mut layout,
                &font,
                max_font_size,
                (max_width, max_height),
                text,
            );

            simple_overlay(
                &mut self.image,
                &mask,
                self.config.color.unwrap_or([0., 0., 0., 1.]),
                bb.min,
            )
        }

        if let Some(watermark) = watermark_msg {
            let (watermark, pos) = render_watermark(
                &mut raster_cache,
                &mut layout,
                &font,
                (self.image.width(), self.image.height()),
                config,
                watermark,
            );

            simple_overlay(
                &mut self.image,
                &watermark,
                self.config.color.unwrap_or([0., 0., 0., 1.]),
                (0, pos),
            )
        }

        self.image
    }
}

impl AnimatedMemeTemplate {
    pub fn render(
        mut self,
        text: &[String],
        config: &Config,
        max_font_size: f32,
        watermark_msg: Option<&str>,
    ) -> Vec<Frame> {
        let font = fontdue::Font::from_bytes(FONT, FontSettings::default()).unwrap();
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);

        let mut raster_cache = HashMap::new();

        let texts: Vec<_> = text
            .iter()
            .zip(&self.config.text)
            .map(|(text, bb)| {
                let max_height = bb.max.1 - bb.min.1;
                let max_width = bb.max.0 - bb.min.0;
                let image = render_text(
                    &mut raster_cache,
                    &mut layout,
                    &font,
                    max_font_size,
                    (max_width, max_height),
                    text,
                );
                (image, bb.min)
            })
            .collect();

        let watermark = watermark_msg.map(|watermark| {
            render_watermark(
                &mut raster_cache,
                &mut layout,
                &font,
                (
                    self.frames[0].buffer().width(),
                    self.frames[0].buffer().height(),
                ),
                config,
                watermark,
            )
        });

        for frame in &mut self.frames {
            let image = frame.buffer_mut();
            for (mask, pos) in &texts {
                simple_overlay(
                    image,
                    &mask,
                    self.config.color.unwrap_or([0., 0., 0., 1.]),
                    *pos,
                )
            }

            if let Some((watermark, pos)) = &watermark {
                simple_overlay(
                    image,
                    &watermark,
                    self.config.color.unwrap_or([0., 0., 0., 1.]),
                    (0, *pos),
                )
            }
        }

        self.frames
    }
}

fn render_glyphs(
    glyphs: &[GlyphPosition],
    raster_cache: &mut HashMap<GlyphRasterConfig, (Metrics, Vec<u8>)>,
    font: &Font,
    mut put_pixel: impl FnMut(u32, u32, u8),
) {
    for glyph in glyphs.iter().filter(|x| !x.char_data.is_control()) {
        let (ref metrics, ref bytes) = raster_cache
            .entry(glyph.key)
            .or_insert_with(|| font.rasterize_config(glyph.key));

        for x in 0..metrics.width {
            for y in 0..metrics.height {
                let coverage = bytes[x + y * metrics.width];
                let x = x as u32 + glyph.x as u32;
                let y = y as u32 + glyph.y as u32;
                put_pixel(x, y, coverage);
            }
        }
    }
}

// TODO: fix oversizing
fn get_filling_glyphs<'a>(
    size: (u32, u32),
    font: &Font,
    layout: &'a mut Layout,
    min_font_size: f32,
    max_font_size: f32,
    text: &str,
) -> &'a [GlyphPosition] {
    let max_width = size.0 as f32;
    let max_height = size.1 as f32;
    let mut min = min_font_size;
    let mut max = max_font_size;

    let abs_max_lines = text.split(char::is_whitespace).count();

    loop {
        let candidate = min + max / 2.;
        layout.reset(&LayoutSettings {
            max_height: Some(max_height),
            max_width: Some(max_width),
            horizontal_align: HorizontalAlign::Center,
            vertical_align: VerticalAlign::Top,
            wrap_style: WrapStyle::Word,
            wrap_hard_breaks: true,
            ..Default::default()
        });
        layout.append(
            &[font],
            &TextStyle {
                text,
                px: candidate,
                font_index: 0,
                user_data: (),
            },
        );
        if layout.lines() > abs_max_lines || layout.height() > max_height {
            max = candidate;
        } else if min - max <= 0.25 {
            break layout.glyphs();
        } else {
            min = candidate;
        }
    }
}

fn render_watermark(
    raster_cache: &mut HashMap<GlyphRasterConfig, (Metrics, Vec<u8>)>,
    layout: &mut Layout,
    font: &Font,
    image_size: (u32, u32),
    config: &Config,
    watermark: &str,
) -> (GrayImage, u32) {
    let (img_width, img_height) = image_size;
    let font_size = img_width.min(img_height) as f32 / config.watermark_size_fraction;
    layout.reset(&LayoutSettings {
        horizontal_align: HorizontalAlign::Left,
        vertical_align: VerticalAlign::Middle,
        ..Default::default()
    });
    layout.append(
        &[font],
        &TextStyle {
            text: watermark,
            px: font_size,
            font_index: 0,
            user_data: (),
        },
    );

    let mut gray_image = GrayImage::from_vec(
        img_width,
        layout.height() as u32,
        vec![0; (img_width * layout.height() as u32) as usize],
    )
    .unwrap();

    render_glyphs(layout.glyphs(), raster_cache, &font, |x, y, coverage| {
        gray_image.put_pixel(x, y, Luma([coverage]));
    });

    (gray_image, img_height - font_size.ceil() as u32)
}

fn render_text(
    raster_cache: &mut HashMap<GlyphRasterConfig, (Metrics, Vec<u8>)>,
    layout: &mut Layout,
    font: &Font,
    max_font_size: f32,
    size: (u32, u32),
    text: &str,
) -> GrayImage {
    let mut gray_image =
        GrayImage::from_vec(size.0, size.1, vec![0; (size.0 * size.1) as usize]).unwrap();

    let glyphs = get_filling_glyphs(size, &font, layout, 5., max_font_size, text);

    render_glyphs(glyphs, raster_cache, &font, |x, y, coverage| {
        gray_image.put_pixel(x, y, Luma([coverage]));
    });

    gray_image
}

fn simple_overlay(image: &mut RgbaImage, mask: &GrayImage, color: [f32; 4], pos: (u32, u32)) {
    for x in 0..mask.width() {
        for y in 0..mask.height() {
            let mask = mask.get_pixel(x, y).0[0] as f32 / u8::MAX as f32;
            let x = pos.0 + x;
            let y = pos.1 + y;

            if (0..image.width()).contains(&x) && (0..image.height()).contains(&y) {
                let prev = image.get_pixel(x, y);
                let [r, g, b, a] = prev.0.map(|x| x as f32 / u8::MAX as f32);

                let zipped = [(r, color[0]), (g, color[1]), (b, color[2]), (a, color[3])];

                let new = zipped
                    .map(|(a, b)| (1. - mask) * a + mask * b)
                    .map(|x| x * u8::MAX as f32)
                    .map(|x| x as u8);
                image.put_pixel(x, y, Rgba(new));
            }
        }
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

#[derive(Serialize, Deserialize, Default)]
struct FileConfig {
    sources: Option<Vec<MemeSource>>,
    watermark: Option<String>,
    watermark_size_fraction: Option<f32>,
}

pub struct Config {
    sources: Vec<MemeSource>,
    watermark: String,
    watermark_size_fraction: f32,
}

impl From<FileConfig> for Config {
    fn from(fc: FileConfig) -> Self {
        Self {
            sources: fc.sources.unwrap_or_else(|| {
                vec![MemeSource::GitUrl {
                    url: "https://github.com/TheRawMeatball/memeinator-memesrc.git".to_owned(),
                    alias: "default".to_owned(),
                }]
            }),
            watermark: fc
                .watermark
                .unwrap_or_else(|| "Made with meme-cli".to_owned()),
            watermark_size_fraction: fc.watermark_size_fraction.unwrap_or(30.),
        }
    }
}

impl Config {
    pub fn load() -> Result<Config, Error> {
        let config_path = dirs::config_dir()
            .ok_or_else(|| anyhow!("config dir not found"))?
            .join("memecli.conf.json");
        match fs::read_to_string(config_path) {
            Ok(config_str) => Ok(serde_json::from_str::<FileConfig>(&config_str)
                .context("The configuration file is broken")?
                .into()),
            Err(_) => Ok(FileConfig::default().into()),
        }
    }

    pub fn watermark(&self) -> &str {
        &self.watermark
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

                    match fs::File::open(dir_path.join("image.png")) {
                        Ok(img) => {
                            let img = image::png::PngDecoder::new(img)?;
                            let image = DynamicImage::from_decoder(img)?.to_rgba8();

                            return Ok(MemeTemplate::Simple(SimpleMemeTemplate { config, image }));
                        }
                        Err(e) => match e.kind() {
                            std::io::ErrorKind::NotFound => {
                                let file = fs::File::open(dir_path.join("animated.gif"))
                                    .with_context(|| {
                                        format!(
                                            "Cannot read animated.gif for format {}",
                                            &template_name
                                        )
                                    })?;

                                let img = image::gif::GifDecoder::new(file)?;
                                let mut frames = vec![];
                                for frame in img.into_frames() {
                                    frames.push(frame?);
                                }
                                return Ok(MemeTemplate::Animated(AnimatedMemeTemplate {
                                    config,
                                    frames,
                                }));
                            }
                            _ => {
                                return Err(e).with_context(|| {
                                    format!("Cannot read image.png for format {}", &template_name)
                                })
                            }
                        },
                    }
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
                if path.is_dir() && path.read_dir()?.next().is_some() {
                    eprintln!("Updating meme repository {} ({})", alias, url);
                    git_ops::update_repo(&path)?;
                } else {
                    eprintln!("Cloning meme repository {} ({})", alias, url);
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
