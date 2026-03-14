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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayOptions {
    pub use_si: bool,
    pub prefer_disk: bool,
    pub show_hidden: bool,
    pub show_item_count: bool,
    pub show_mtime: bool,
    pub show_percent: bool,
    pub show_graph: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            use_si: false,
            prefer_disk: false,
            show_hidden: false,
            show_item_count: false,
            show_mtime: false,
            show_percent: true,
            show_graph: true,
        }
    }
}
