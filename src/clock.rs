// uDashBoard: featherweight dashboard application.
//
// Copyright (C) 2019  Brandon Lewis
//
// This program is free software: you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public License
// as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this program.  If not, see
// <https://www.gnu.org/licenses/>.

use std::time::Instant;

// Wrapper around somewhat obnoxious system time api.
pub struct Clock {
    instant: Instant,
}

impl Clock {
    pub fn new() -> Clock {
        Clock {
            instant: Instant::now(),
        }
    }

    // Return system time as floating point value.
    pub fn seconds(&self) -> f64 {
        let e = self.instant.elapsed();
        (e.as_secs() as f64) + (0.001 * e.subsec_millis() as f64)
    }
}
