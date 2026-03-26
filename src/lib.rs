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

pub mod app;
pub mod events;
pub mod ui;

pub use app::cli;
pub use app::config;
pub use app::input;
pub use app::scan;
pub use app::scan::accumulator as scan_accumulator;
pub use app::scan::control as scan_control;
pub use app::scan::manager as scan_manager;
pub use app::scan::scanner as fs_scan;
pub use app::size;
pub use app::snapshot;
pub use app::state;
pub use app::tree;
pub use ui::display;
pub use ui::theme;
pub use ui::treemap;
