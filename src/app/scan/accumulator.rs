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

use crate::scan::scanner::ScanNode;

#[derive(Debug, Default)]
pub struct ScanAccumulator {
    nodes: Vec<ScanNode>,
}

impl ScanAccumulator {
    pub fn push_batch(&mut self, nodes: Vec<ScanNode>) {
        self.nodes.extend(nodes);
    }

    pub fn push_node(&mut self, node: ScanNode) {
        self.nodes.push(node);
    }

    pub fn drain(&mut self) -> Vec<ScanNode> {
        self.nodes.drain(..).collect()
    }
}
