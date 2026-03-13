use anyhow::Result;
use dar::{app, cli::CliCommand, config};

fn run() -> Result<()> {
    match CliCommand::parse()? {
        CliCommand::Run(cli_args) => {
            let config_load = config::load(cli_args.ignore_config);
            app::run(cli_args, config_load)?;
        }
        CliCommand::Help => {
            println!("{}", CliCommand::help_text());
            return Ok(());
        }
        CliCommand::Version => {
            println!("{}", CliCommand::version_text());
            return Ok(());
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
