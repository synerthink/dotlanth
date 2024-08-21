/// VMError Module
///
/// This module contains the `VMError` enum which represents the possible errors that can occur
/// in the virtual machine.
pub mod vm_error;

/// InstructionError Module
///
/// This module contains the `InstructionError` enum which represents the possible errors that can occur
/// during the execution of an instruction.
pub mod instruction_error;

pub use vm_error::VMError;
pub use instruction_error::InstructionError;