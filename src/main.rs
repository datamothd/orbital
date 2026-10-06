mod sysinfo;
#[path = "tweaks/tweaks.rs"]
mod tweaks;

use clap::{Parser, Subcommand};
use std::io;

#[derive(Parser)]
#[command(
    name = "orbital",
    about = "Basic framework of a Windows optimization tool. Only displays system information and Windows preferences for the time being."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Sysinfo,
    TaskbarAlignment {
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

fn main() -> io::Result<()> {
    match Cli::parse().command {
        Command::TaskbarAlignment { yes } => tweaks::toggle_taskbar_alignment(yes)?,
        Command::Sysinfo => sysinfo::show(),
    }
    Ok(())
}
