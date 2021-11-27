# meme-cli

A command line utility to easily make dank memes.

Yes, really.

## Installation

Currently, the only way to install this is by cloning the repository and building manually. AUR packages and a crates.io release are planned.

```sh
git clone https://github.com/TheRawMeatball/meme-cli
cd meme-cli
cargo install --path meme-cli
```

## Usage Example

```sh
# make sure to update your sources after installation, and then again regularly 
meme-cli update-sources
meme-cli generate gru-plan "make memecli" "get it working enough to release it" "you need to write a readme" "you need to write a readme"
```

will generate the following meme, and put it on your clipboard to easily share it:

![(the meme you generated)](resources/example.png)

Note: if you don't get the meme on your clipboard, you might need to install a clipboard manager or enable image support on your clipboard manager.

## Tips and tricks

You can run `meme-cli generate-proto-completions` to generate some rough completion scripts for your preferred shell. You can install them directly using the instructions for your shell, but I'd recommend extending them to support template completions for `meme-cli generate`. As an example, if you're using fish, it means adding this line to your completion file:

```fish
complete -c meme-cli -n "__fish_seen_subcommand_from generate" -a "(meme-cli list-templates)"
```

## What's all the other crates then???

Glad you asked! `meme-cli` is but a frontend for the true meme generation powerhouse, `memeinator`. `meme-bevy` is a different frontend, but it's used for quickly making the meme templates used by `meme-cli` instead. You can use it by configuring a local meme repository in `~/.config/memecli.conf.json`. The templates you add will go there. If you think others would like them, feel free to make a PR to [the official meme repository](https://github.com/TheRawMeatball/memeinator-memesrc).

```json
{
  "sources": [
    {
      "GitUrl": {
        "url": "https://github.com/TheRawMeatball/memeinator-memesrc.git",
        "alias": "default"
      }
    },
    {
      "LocalPath": "/home/your-username/memes"
    }
  ]
}
```

## License

I don't know why you'd be interested in the license of such a joke, but if you must, it's dual licensed under MIT and Apache 2.0.
