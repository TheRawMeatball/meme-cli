use std::path::PathBuf;

use anyhow::{anyhow, Error};
use memeinator::{Config, MemeConfig, MemeText};
use structopt::{clap::Shell, StructOpt};

mod image_io;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "meme-cli",
    about = "A way to easily generate dank memes from preconfigured templates"
)]
enum Opt {
    Generate(Generate),
    MakeTemplate(MakeTemplate),
    #[structopt(about = "List all template sources")]
    ListSources,
    #[structopt(about = "List all template names")]
    ListTemplates,
    #[structopt(about = "Fetch potential new memes from the configured sources")]
    UpdateSources,
    #[structopt(about = "Generates a basic completion script")]
    GenerateProtoCompletions(GenerateProtoCompletions),
}

#[derive(Debug, StructOpt)]
enum GenerateProtoCompletions {
    Bash,
    Zsh,
    Fish,
}

#[derive(Debug, StructOpt)]
#[structopt(name = "gen", about = "Generate a meme from a template")]
struct Generate {
    /// The template to use
    template: String,
    /// The text placed into the template
    inputs: Vec<String>,

    /// The output path for the meme. By default, the meme will be pushed to the clipboard.
    #[structopt(short, long)]
    output: Option<PathBuf>,

    /// The maximum font size for the text. Defaults to 600.
    #[structopt(short, long)]
    max_size: Option<f32>,

    // Disables adding the watermark
    #[structopt(short, long)]
    no_watermark: bool,
}

impl Generate {
    fn run(self, config: Config) -> Result<(), Error> {
        let meme = config.get_meme_template(&self.template)?;
        println!("Template found");
        let img_buffer = meme.render(
            &self.inputs,
            &config,
            self.max_size.unwrap_or(600.),
            !self.no_watermark,
        )?;
        println!("Meme rendered");

        if let Some(out_path) = self.output {
            img_buffer.save(out_path)?;
        } else {
            image_io::image_out(&img_buffer)?;
        }
        println!("Done!");
        Ok(())
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "make-template", about = "Generate a meme template and save it")]
struct MakeTemplate {
    /// The image path for the meme. Can be a URL. If absent, an image will be pulled from the clipboard
    #[structopt(short, long)]
    input: Option<PathBuf>,

    /// The template name
    template_name: String,

    /// The coordinates for text, given in `LEFT-TOP-RIGHT-BOTTOM`
    coordinates: Vec<String>,
}

impl MakeTemplate {
    fn run(self, config: Config) -> Result<(), Error> {
        let img = if let Some(path) = self.input {
            image::open(path)?.to_rgba8()
        } else {
            image_io::image_in()?
        };
        let mut coords = Vec::with_capacity(self.coordinates.len());
        for coord in self.coordinates {
            let mut iterator = coord.split("-").map(str::parse::<u32>);
            let e = || anyhow!("Incorrect coordinate literal");
            let text = MemeText {
                min: (
                    iterator.next().ok_or_else(e)??,
                    iterator.next().ok_or_else(e)??,
                ),
                max: (
                    iterator.next().ok_or_else(e)??,
                    iterator.next().ok_or_else(e)??,
                ),
            };
            coords.push(text);
        }
        let meme_config = MemeConfig {
            color: Some([0., 0., 0., 1.]),
            text: coords,
        };
        config.write_template(
            img.as_raw(),
            img.width(),
            img.height(),
            meme_config,
            &self.template_name,
        )
    }
}

fn main() -> Result<(), Error> {
    let config = Config::load()?;
    match Opt::from_args() {
        Opt::Generate(generate) => generate.run(config),
        Opt::MakeTemplate(make_template) => make_template.run(config),
        Opt::ListSources => list_sources(config),
        Opt::ListTemplates => list_templates(config),
        Opt::UpdateSources => update_sources(config),
        Opt::GenerateProtoCompletions(completions) => {
            let shell = match completions {
                GenerateProtoCompletions::Bash => Shell::Bash,
                GenerateProtoCompletions::Zsh => Shell::Zsh,
                GenerateProtoCompletions::Fish => Shell::Fish,
            };
            Opt::clap().gen_completions_to("meme-cli", shell, &mut std::io::stdout());
            Ok(())
        }
    }
}

fn list_sources(config: Config) -> Result<(), Error> {
    for source in config.fetch_source_list() {
        match source {
            memeinator::MemeSource::GitUrl { url, alias } => {
                println!("Git source {} (URL: {})", alias, url)
            }
            memeinator::MemeSource::LocalPath(path) => println!("Local source @ {}", path),
        }
    }
    Ok(())
}

fn update_sources(config: Config) -> Result<(), Error> {
    for source in config.fetch_source_list() {
        source.to_path_and_update()?;
    }
    Ok(())
}

fn list_templates(config: Config) -> Result<(), Error> {
    for template in config.fetch_template_list() {
        println!("{}", template)
    }
    Ok(())
}
