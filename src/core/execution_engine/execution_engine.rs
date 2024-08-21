use crate::{
    contracts::SimpleAdditionContract,
    core::execution_engine::{
        errors::InstructionError,
        instructions::{
            ArithmeticInstruction, ControlFlowInstruction, InstructionProcessor, MemoryInstruction,
        },
    },
};
use log::{debug, info};

use super::errors::VMError;

pub struct ExecutionEngine;

impl ExecutionEngine {
    pub fn execute_contract(contract: &SimpleAdditionContract) -> Result<i32, VMError> {
        debug!("Input parameters: a={}, b={}", contract.a, contract.b);

        let mut processor = InstructionProcessor::new();

        processor.stack.push(contract.a);
        processor.stack.push(contract.b);
        ArithmeticInstruction::Add.execute(&mut processor)?;
        let add_result = processor.stack.pop().unwrap();
        info!("Executing instruction: ADD, result = {}", add_result);

        processor.stack.push(contract.a);
        processor.stack.push(contract.b);
        ArithmeticInstruction::Sub.execute(&mut processor)?;
        let sub_result = processor.stack.pop().unwrap();
        info!("Executing instruction: SUB, result = {}", sub_result);

        processor.stack.push(contract.a);
        processor.stack.push(0); // This should trigger DivisionByZero
        match ArithmeticInstruction::Div.execute(&mut processor) {
            Ok(_) => {
                let div_result = processor.stack.pop().unwrap();
                info!("Executing instruction: DIV, result = {}", div_result);
            }
            Err(InstructionError::DivisionByZero) => {
                info!("Executing instruction: DIV, result = Division by zero error");
            }
            Err(e) => return Err(e.into()),
        }

        processor.stack.push(10);
        ControlFlowInstruction::Jump.execute(&mut processor)?;
        info!("Executing instruction: JUMP to address 10");

        processor.stack.push(1);
        processor.stack.push(20);
        ControlFlowInstruction::JumpIf.execute(&mut processor)?;
        info!("Executing instruction: JUMPIF to address 20");

        processor.memory.insert(30, 999);
        processor.stack.push(30);
        MemoryInstruction::Load.execute(&mut processor)?;
        let load_result = processor.stack.pop().unwrap();
        info!(
            "Executing instruction: LOAD from address 30, result = {}",
            load_result
        );

        processor.stack.push(42);
        processor.stack.push(40);
        MemoryInstruction::Store.execute(&mut processor)?;
        info!("Executing instruction: STORE value 42 at address 40");

        Ok(add_result)
    }
}
