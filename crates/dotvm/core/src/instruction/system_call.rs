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

// NOTE: The implementation of the system call instructions is incomplete and it's just a placeholder.
use crate::instruction::instruction::Instruction;
use crate::vm::errors::VMError;
use crate::vm::executor::Executor;
use std::io::{self, Write};
use std::process::Command;

pub struct WriteSysCallInstruction;

impl WriteSysCallInstruction {
    pub fn new() -> Self {
        WriteSysCallInstruction
    }
}

impl Instruction for WriteSysCallInstruction {
    fn execute(&self, executor: &mut Executor) -> Result<(), VMError> {
        // Pop a value and print it to the console.
        let value = executor.pop_operand()?;
        println!("{}", value);
        Ok(())
    }
}

pub struct ReadSysCallInstruction;

impl ReadSysCallInstruction {
    pub fn new() -> Self {
        ReadSysCallInstruction
    }
}

impl Instruction for ReadSysCallInstruction {
    fn execute(&self, executor: &mut Executor) -> Result<(), VMError> {
        // Read a number from the console input and push it onto the operand stack.
        print!("Enter a number: ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|_| VMError::SystemCallError("Failed to read from stdin".into()))?;
        let parsed: f64 = input.trim().parse().map_err(|_| VMError::SystemCallError("Invalid number entered".into()))?;
        executor.push_operand(parsed);
        Ok(())
    }
}

pub struct CreateProcessInstruction {
    command: String,
}

impl CreateProcessInstruction {
    pub fn new(command: String) -> Self {
        CreateProcessInstruction { command }
    }
}

impl Instruction for CreateProcessInstruction {
    fn execute(&self, executor: &mut Executor) -> Result<(), VMError> {
        // Spawn a new process using the specified command.
        let child = Command::new(&self.command)
            .spawn()
            .map_err(|e| VMError::ProcessError(format!("Failed to spawn process '{}': {}", self.command, e)))?;
        // Push the process ID (converted to f64) onto the operand stack.
        executor.push_operand(child.id() as f64);
        Ok(())
    }
}

pub struct TerminateProcessInstruction {
    pid: u32,
}

impl TerminateProcessInstruction {
    pub fn new(pid: u32) -> Self {
        TerminateProcessInstruction { pid }
    }
}

impl Instruction for TerminateProcessInstruction {
    fn execute(&self, _executor: &mut Executor) -> Result<(), VMError> {
        // For demonstration, we print the termination request.
        // A real implementation would call system APIs (or use external crates) to terminate the process.
        println!("Terminating process with pid: {}", self.pid);
        Ok(())
    }
}

pub struct SendNetworkPacketInstruction {
    remote_addr: String,
    port: u16,
}

impl SendNetworkPacketInstruction {
    pub fn new(remote_addr: String, port: u16) -> Self {
        SendNetworkPacketInstruction { remote_addr, port }
    }
}

impl Instruction for SendNetworkPacketInstruction {
    fn execute(&self, executor: &mut Executor) -> Result<(), VMError> {
        // Pop data from the operand stack to be “sent”.
        let data = executor.pop_operand()?;
        // In a real implementation, you might open a socket and send data.
        println!("Sending {} to {}:{}", data, self.remote_addr, self.port);
        Ok(())
    }
}

pub struct ReceiveNetworkPacketInstruction {
    port: u16,
}

impl ReceiveNetworkPacketInstruction {
    pub fn new(port: u16) -> Self {
        ReceiveNetworkPacketInstruction { port }
    }
}

impl Instruction for ReceiveNetworkPacketInstruction {
    fn execute(&self, executor: &mut Executor) -> Result<(), VMError> {
        // Simulate reception of a network packet.
        println!("Receiving packet on port: {}", self.port);
        // Push dummy data onto the stack.
        executor.push_operand(42.0);
        Ok(())
    }
}
