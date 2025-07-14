use std::sync::Arc;

use onion_frontend::{compile::build_code, utils::cycle_detector};
use onion_vm::{
    lambda::{
        runnable::{Runnable, RuntimeError, StepResult},
        scheduler::scheduler::Scheduler,
    },
    types::{
        lambda::{
            definition::{LambdaBody, OnionLambdaDefinition},
            launcher::OnionLambdaRunnableLauncher,
            vm_instructions::{
                instruction_set::VMInstructionPackage, ir::IRPackage, ir_translator::IRTranslator,
            },
        },
        named::OnionNamed,
        object::{OnionObject, OnionStaticObject},
        tuple::OnionTuple,
    },
    unwrap_object, GC,
};

mod stdlib;
pub use arc_gc;
pub use onion_frontend;
pub use onion_vm;
pub use stdlib::build_named_dict;
pub use stdlib::get_attr_direct;
pub use stdlib::wrap_native_function;

// Import necessary items for async
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::time::{sleep, Duration};

pub async fn eval(
    code: &str,
    dir_stack: &mut onion_frontend::dir_stack::DirectoryStack,
    context: Option<Vec<&OnionStaticObject>>,
) -> Result<OnionStaticObject, String> {
    // Execute the code and return the result
    let mut cycle_detector = cycle_detector::CycleDetector::new();
    execute_code(code, &mut cycle_detector, dir_stack, context).await
}

async fn execute_code(
    code: &str,
    cycle_detector: &mut cycle_detector::CycleDetector<String>,
    dir_stack: &mut onion_frontend::dir_stack::DirectoryStack,
    context: Option<Vec<&OnionStaticObject>>,
) -> Result<OnionStaticObject, String> {
    let ir_package = build_code(code, cycle_detector, dir_stack)
        .map_err(|e| format!("Compilation failed: {}", e))?;

    execute_ir_package(&ir_package, context).await
}

async fn execute_ir_package(
    ir_package: &IRPackage,
    context: Option<Vec<&OnionStaticObject>>,
) -> Result<OnionStaticObject, String> {
    let mut translator = IRTranslator::new(ir_package);
    translator
        .translate()
        .map_err(|e| format!("IR translation failed: {:?}", e))?;

    let vm_instructions_package = translator.get_result();
    execute_bytecode_package(&vm_instructions_package, context).await
}

// Modify execute_bytecode_package to be async
pub async fn execute_bytecode_package(
    vm_instructions_package: &VMInstructionPackage,
    context: Option<Vec<&OnionStaticObject>>,
) -> Result<OnionStaticObject, String> {
    let mut gc = GC::new_with_memory_threshold(1024 * 1024); // 1 MB threshold

    match VMInstructionPackage::validate(vm_instructions_package) {
        Err(e) => return Err(format!("Invalid VM instruction package: {}", e)),
        Ok(_) => {}
    }
    // Create standard library object
    let stdlib_pair = OnionNamed::new_static(
        &OnionObject::String(Arc::new("stdlib".to_string())).consume_and_stabilize(),
        &stdlib::build_module(),
    );

    // Create Lambda definition
    let lambda = match context {
        Some(ref ctx) => {
            let mut params = ctx.clone();
            params.push(&stdlib_pair);
            OnionLambdaDefinition::new_static(
                &OnionTuple::new_static(params),
                LambdaBody::Instruction(Arc::new(vm_instructions_package.clone())),
                None,
                None,
                "__main__".to_string(),
            )
        }
        None => OnionLambdaDefinition::new_static(
            &OnionTuple::new_static(vec![&stdlib_pair]),
            LambdaBody::Instruction(Arc::new(vm_instructions_package.clone())),
            None,
            None,
            "__main__".to_string(),
        ),
    };

    let args = OnionTuple::new_static(vec![]);

    // 初始化调度器和GC
    let mut scheduler: Box<dyn Runnable> = Box::new(
        OnionLambdaRunnableLauncher::new_static(&lambda, &args, |r| {
            Ok(Box::new(Scheduler::new(vec![r])))
        })
        .map_err(|e| format!("Failed to create runnable Lambda: {:?}", e))?,
    );
    // Execute code
    loop {
        match scheduler.step(&mut gc) {
            StepResult::Continue => {
                // Continue to next step
                // Yield control back to the async runtime
                sleep(Duration::from_secs(0)).await;
            }
            StepResult::SetSelfObject(_) => {
                return Err("Invalid operation: SetSelfObject is not supported".to_string());
            }
            StepResult::SpawnRunnable(_) => {
                return Err("Invalid operation: SpawnRunnable is not supported".to_string());
            }
            StepResult::Error(ref error) => {
                return Err(format!("Execution error: {}", error));
            }
            StepResult::NewRunnable(_) => {
                return Err("Invalid operation: NewRunnable is not supported".to_string());
            }
            StepResult::ReplaceRunnable(ref r) => {
                scheduler = r.copy();
                // Yield control after replacing runnable
                sleep(Duration::from_secs(0)).await;
            }
            StepResult::Return(ref result) => {
                let result_borrowed = result.weak();
                let result = unwrap_object!(result_borrowed, OnionObject::Pair)
                    .map_err(|e| format!("Failed to unwrap result: {:?}", e))?;
                let success = *unwrap_object!(result.get_key(), OnionObject::Boolean)
                    .map_err(|e| format!("Failed to get success key: {:?}", e))?;
                if !success {
                    return Err(result
                        .get_value()
                        .to_string(&vec![])
                        .map_err(|e| format!("Failed to get error message: {:?}", e))?);
                }
                return Ok(result.get_value().clone().stabilize());
            }
        }
    }
}
