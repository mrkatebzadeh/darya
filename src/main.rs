use anyhow::Result;
use dar::{app, cli::CliCommand, config};

fn run() -> Result<()> {
    match CliCommand::parse()? {
        CliCommand::Run(cli_args) => {
            let config_load = config::load();
            app::run(cli_args, config_load)?;
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
