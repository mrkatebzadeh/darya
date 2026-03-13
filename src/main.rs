mod cli;

use anyhow::Result;
use cli::CliCommand;

fn run() -> Result<()> {
    match CliCommand::parse()? {
        CliCommand::Run(cli_args) => {
            println!("starting dar for {}", cli_args.root.display());
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
