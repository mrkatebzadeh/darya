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

use crate::state::{AppState, StatusMessage};

pub(crate) fn start_filter(state: &mut AppState) {
    state.filter_active = true;
    state.filter_prompt_active = true;
    state.update_status(StatusMessage::FilterPrompt);
}

pub(crate) fn filter_char(state: &mut AppState, ch: char) {
    state.filter_query.push(ch);
    state.filter_active = true;
}

pub(crate) fn filter_backspace(state: &mut AppState) {
    state.filter_query.pop();
}

pub(crate) fn apply_filter(state: &mut AppState) {
    state.filter_prompt_active = false;
    if state.filter_query.is_empty() {
        state.filter_active = false;
        state.update_status(StatusMessage::FilterCleared);
    } else {
        state.filter_active = true;
        state.update_status(StatusMessage::FilterActive(state.filter_query.clone()));
    }
}

pub(crate) fn clear_filter(state: &mut AppState) {
    state.filter_prompt_active = false;
    state.clear_filter();
    state.update_status(StatusMessage::FilterCleared);
}
