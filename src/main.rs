use orbital::{explorer, sysinfo};

use clap::{Parser, Subcommand};
use std::io;

#[derive(Parser)]
#[command(
    name = "orbital",
    about = "Basic framework of a Windows optimization tool. Only displays system information and changes a few Windows preferences for the time being."
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
    ExplorerCompactMode {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    ClassicContextMenu {
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

fn main() -> io::Result<()> {
    match Cli::parse().command {
        Command::TaskbarAlignment { yes } => explorer::toggle_taskbar_alignment(yes)?,
        Command::ExplorerCompactMode { yes } => explorer::toggle_explorer_compact_mode(yes)?,
        Command::ClassicContextMenu { yes } => explorer::toggle_classic_context_menu(yes)?,
        Command::Sysinfo => sysinfo::show()?,
    }
    Ok(())
}
