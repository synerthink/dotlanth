use crate::core::execution_engine::errors::InstructionError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Opcode {
    LoadNumber,
    Add,
    Sub,
    Mul,
    Div,
    Jump,
    JumpIf,
    LoadFromMemory,
    StoreToMemory,
    CallFunction,
}

impl From<Opcode> for u8 {
    fn from(opcode: Opcode) -> Self {
        match opcode {
            Opcode::LoadNumber => 0x01,
            Opcode::Add => 0x02,
            Opcode::Sub => 0x03,
            Opcode::Mul => 0x04,
            Opcode::Div => 0x05,
            Opcode::Jump => 0x05,
            Opcode::JumpIf => 0x06,
            Opcode::LoadFromMemory => 0x08,
            Opcode::StoreToMemory => 0x09,
            Opcode::CallFunction => 0x10,
        }
    }
}

impl TryFrom<u8> for Opcode {
    type Error = InstructionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Opcode::LoadNumber),
            0x02 => Ok(Opcode::Add),
            0x03 => Ok(Opcode::Sub),
            0x04 => Ok(Opcode::Mul),
            0x05 => Ok(Opcode::Div),
            0x06 => Ok(Opcode::Jump),
            0x07 => Ok(Opcode::JumpIf),
            0x08 => Ok(Opcode::LoadFromMemory),
            0x09 => Ok(Opcode::StoreToMemory),
            0x10 => Ok(Opcode::CallFunction),
            _ => Err(InstructionError::InvalidOpcode),
        }
    }
}
