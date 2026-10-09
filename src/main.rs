use orbital::{explorer, sysinfo};

use clap::{Parser, Subcommand};
use std::io;

#[derive(Parser)]
#[command(
    name = "orbital",
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Sysinfo,
    DisableTaskbarCentering {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    DisableExplorerSpacing {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    DisableModernContextMenu {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    HideRecentFiles {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    HideFrequentFolders {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    HideOfficeFiles {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    HideHomeFolder {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    HideGallery {
        #[arg(short = 'y', long)]
        yes: bool,
    },
    HideShortcutArrow {
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

fn main() -> io::Result<()> {
    match Cli::parse().command {
        Command::DisableTaskbarCentering { yes } => {
            explorer::toggle_disable_taskbar_centering(yes)?
        }
        Command::DisableExplorerSpacing { yes } => explorer::toggle_disable_explorer_spacing(yes)?,
        Command::DisableModernContextMenu { yes } => {
            explorer::toggle_disable_modern_context_menu(yes)?
        }
        Command::HideRecentFiles { yes } => explorer::toggle_hide_recent_files(yes)?,
        Command::HideFrequentFolders { yes } => explorer::toggle_hide_frequent_folders(yes)?,
        Command::HideOfficeFiles { yes } => explorer::toggle_hide_office_files(yes)?,
        Command::HideHomeFolder { yes } => explorer::toggle_hide_home_folder(yes)?,
        Command::HideGallery { yes } => explorer::toggle_hide_gallery(yes)?,
        Command::HideShortcutArrow { yes } => explorer::toggle_hide_shortcut_arrow(yes)?,
        Command::Sysinfo => sysinfo::show()?,
    }
    Ok(())
}
