use clap::{Parser, Subcommand};
use orbital::{explorer, sysinfo};
use std::io;

#[derive(Parser)]
#[command(name = "orbital")]
struct Cli {
    #[arg(short = 'y', long, global = true)]
    yes: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Sysinfo,
    ApplyRecommendedSettings,
    #[command(flatten)]
    Explorer(explorer::Preference),
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::ApplyRecommendedSettings => explorer::apply_recommended(cli.yes),
        Command::Explorer(preference) => explorer::toggle_preference(preference, cli.yes, true),
        Command::Sysinfo => sysinfo::show(),
    }
}
