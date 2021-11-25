use std::{borrow::Cow, path::PathBuf};

use anyhow::Error;
use arboard::ImageData;
use memeinator::Config;
use structopt::{clap::Shell, StructOpt};

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
}

impl Generate {
    fn run(self, config: Config) -> Result<(), Error> {
        let meme = config.get_meme_template(&self.template)?;

        let img_buffer = meme.render(&self.inputs, &config)?;
        let mut clipboard = arboard::Clipboard::new()?;

        if let Some(out_path) = self.output {
            img_buffer.save(out_path)?;
        } else {
            clipboard.set_image(ImageData {
                width: img_buffer.width() as _,
                height: img_buffer.height() as _,
                bytes: Cow::Borrowed(&img_buffer),
            })?;
        }

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
    template: String,

    /// The coordinates for text, given in `LEFT-TOP-RIGHT-BOTTOM`
    coordinates: Vec<String>,
}

impl MakeTemplate {
    fn run(self, config: Config) -> Result<(), Error> {
        Err(anyhow::anyhow!("not implemented yet, sowwy :3"))
    }
}

fn main() -> Result<(), Error> {
    let config = Config::load()?;
    match Opt::from_args() {
        Opt::Generate(generate) => generate.run(config),
        Opt::MakeTemplate(make_template) => make_template.run(config),
        Opt::ListSources => list_sources(config),
        Opt::ListTemplates => list_templates(config),
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

fn list_templates(config: Config) -> Result<(), Error> {
    for template in config.fetch_template_list() {
        println!("{}", template)
    }
    Ok(())
}
