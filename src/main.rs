// Copyright (C) 2026 M.R. Siavash Katebzadeh <mr@katebzadeh.xyz>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use anyhow::Result;
use dar::{app, cli::DarCli, config};

fn run() -> Result<()> {
    let dar_cli = match DarCli::try_parse() {
        Ok(c) => c,
        Err(e) => e.exit(),
    };

    let cli_args = dar_cli
        .into_cli_args()
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let config_load = config::load(cli_args.ignore_config);
    app::run(cli_args, config_load)?;

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("dar: {err}");
        std::process::exit(1);
    }
}
