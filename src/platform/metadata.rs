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

use std::fs::Metadata;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

#[cfg(windows)]
use std::os::windows::fs::MetadataExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DeviceId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HardLinkKey {
    device: DeviceId,
    index: u64,
}

impl HardLinkKey {
    pub fn new(device: DeviceId, index: u64) -> Self {
        Self { device, index }
    }
}

#[cfg(unix)]
pub fn device_id(metadata: &Metadata) -> Option<DeviceId> {
    Some(DeviceId(metadata.dev()))
}

#[cfg(windows)]
pub fn device_id(metadata: &Metadata) -> Option<DeviceId> {
    Some(DeviceId(metadata.volume_serial_number() as u64))
}

#[cfg(not(any(unix, windows)))]
pub fn device_id(_: &Metadata) -> Option<DeviceId> {
    None
}

#[cfg(unix)]
pub fn hard_link_key(metadata: &Metadata) -> Option<HardLinkKey> {
    Some(HardLinkKey::new(DeviceId(metadata.dev()), metadata.ino()))
}

#[cfg(windows)]
pub fn hard_link_key(metadata: &Metadata) -> Option<HardLinkKey> {
    Some(HardLinkKey::new(
        DeviceId(metadata.volume_serial_number() as u64),
        metadata.file_index(),
    ))
}

#[cfg(not(any(unix, windows)))]
pub fn hard_link_key(_: &Metadata) -> Option<HardLinkKey> {
    None
}

#[cfg(unix)]
pub fn disk_usage_bytes(metadata: &Metadata) -> u64 {
    metadata.blocks() * 512
}

#[cfg(windows)]
pub fn disk_usage_bytes(metadata: &Metadata) -> u64 {
    metadata.allocation_size()
}

#[cfg(not(any(unix, windows)))]
pub fn disk_usage_bytes(metadata: &Metadata) -> u64 {
    metadata.len()
}
