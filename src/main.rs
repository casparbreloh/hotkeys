use clap::Parser;

fn main() -> anyhow::Result<()> {
    hotkeys::Cli::parse().run()
}
