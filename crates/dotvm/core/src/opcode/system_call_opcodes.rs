// Dotlanth
// Copyright (C) 2025 Synerthink

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum SystemCallOpcode {
    Write = 0x30,
    Read = 0x31,
    CreateProcess = 0x32,
    TerminateProcess = 0x33,
    NetworkSend = 0x34,
    NetworkReceive = 0x35,
}

impl SystemCallOpcode {
    pub fn from_mnemonic(mnemonic: &str) -> Option<Self> {
        match mnemonic {
            "SYSCALL_WRITE" => Some(SystemCallOpcode::Write),
            "SYSCALL_READ" => Some(SystemCallOpcode::Read),
            "SYSCALL_CREATE_PROCESS" => Some(SystemCallOpcode::CreateProcess),
            "SYSCALL_TERMINATE_PROCESS" => Some(SystemCallOpcode::TerminateProcess),
            "SYSCALL_NETWORK_SEND" => Some(SystemCallOpcode::NetworkSend),
            "SYSCALL_NETWORK_RECEIVE" => Some(SystemCallOpcode::NetworkReceive),
            _ => None,
        }
    }

    pub fn to_mnemonic(&self) -> &'static str {
        match self {
            SystemCallOpcode::Write => "SYSCALL_WRITE",
            SystemCallOpcode::Read => "SYSCALL_READ",
            SystemCallOpcode::CreateProcess => "SYSCALL_CREATE_PROCESS",
            SystemCallOpcode::TerminateProcess => "SYSCALL_TERMINATE_PROCESS",
            SystemCallOpcode::NetworkSend => "SYSCALL_NETWORK_SEND",
            SystemCallOpcode::NetworkReceive => "SYSCALL_NETWORK_RECEIVE",
        }
    }
}

impl std::fmt::Display for SystemCallOpcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_mnemonic())
    }
}
