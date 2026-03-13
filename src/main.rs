mod cli;
mod config;

use anyhow::Result;
use cli::CliCommand;

fn run() -> Result<()> {
    let config_load = config::load();

    match CliCommand::parse()? {
        CliCommand::Run(cli_args) => {
            if let Some(err) = config_load.error() {
                eprintln!("config: {err}");
            }

            println!(
                "starting dar for {} using config {}",
                cli_args.root.display(),
                config_load.source_description(),
            );
        }
        CliCommand::Help => {
            println!("{}", CliCommand::help_text());
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("dar: {err}");
        std::process::exit(1);
    }
}
