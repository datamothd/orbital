use clap::{Arg, CommandFactory, FromArgMatches, Parser, Subcommand};
use orbital::{explorer, sysinfo};
use std::io;

#[derive(Parser)]
#[command(
    name = "orbital",
    after_help = "Settings commands share one preview, confirmation, and Explorer restart."
)]
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
    let mut parser = Cli::command();
    for command in parser.get_subcommands_mut() {
        *command = command
            .clone()
            .arg(Arg::new("commands").value_name("COMMAND").num_args(0..));
    }
    let matches = parser.get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|error| error.exit());
    let additional = matches
        .subcommand()
        .and_then(|(_, args)| args.get_many::<String>("commands"));
    let commands = std::iter::once(cli.command)
        .chain(
            additional
                .into_iter()
                .flatten()
                .map(|command| Cli::parse_from(["orbital", command]).command),
        )
        .collect::<Vec<_>>();
    let mut batch = Vec::new();
    for command in commands {
        match command {
            Command::ApplyRecommendedSettings => batch.push(explorer::SettingsCommand::Recommended),
            Command::Explorer(preference) => {
                batch.push(explorer::SettingsCommand::Toggle(preference));
            }
            Command::Sysinfo => sysinfo::show()?,
        }
    }
    if !batch.is_empty() {
        explorer::apply_batch(&batch, cli.yes)?;
    }
    Ok(())
}
