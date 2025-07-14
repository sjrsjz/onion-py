use indexmap::IndexMap;
use onion_vm::{
    lambda::runnable::{Runnable, RuntimeError, StepResult},
    onion_tuple,
    types::{
        lambda::definition::{LambdaBody, OnionLambdaDefinition},
        named::OnionNamed,
        object::{OnionObject, OnionObjectCell, OnionStaticObject},
        tuple::OnionTuple,
    },
    unwrap_step_result, GC,
};

mod bytes;
mod math;
mod string;
mod time;
mod tuple;
mod types;

pub fn build_named_dict(dict: IndexMap<String, OnionStaticObject>) -> OnionStaticObject {
    let mut pairs = vec![];
    for (key, value) in dict {
        pairs.push(OnionNamed::new_static(
            &OnionObject::String(key.into()).stabilize(),
            &value,
        ));
    }
    OnionTuple::new_static_no_ref(&pairs)
}

pub fn get_attr_direct(obj: &OnionObject, key: String) -> Result<OnionStaticObject, RuntimeError> {
    obj.with_attribute(&OnionObject::String(key.into()), &|obj| Ok(obj.stabilize()))
}

pub struct NativeFunctionGenerator<F>
where
    F: Fn(&OnionStaticObject, &mut GC<OnionObjectCell>) -> Result<OnionStaticObject, RuntimeError>
        + 'static,
{
    argument: OnionStaticObject,
    self_object: Option<OnionStaticObject>,
    function: &'static F,
}

impl<F> Runnable for NativeFunctionGenerator<F>
where
    F: Fn(&OnionStaticObject, &mut GC<OnionObjectCell>) -> Result<OnionStaticObject, RuntimeError>
        + Send
        + Sync
        + 'static,
{
    fn step(&mut self, gc: &mut GC<OnionObjectCell>) -> StepResult {
        unwrap_step_result!(
            (self.function)(&self.argument, gc).map(|result| StepResult::Return(result.into()))
        )
    }

    fn receive(
        &mut self,
        step_result: &StepResult,
        _gc: &mut GC<OnionObjectCell>,
    ) -> Result<(), RuntimeError> {
        match step_result {
            StepResult::Return(result) => {
                self.argument = result.as_ref().clone();
                Ok(())
            }
            StepResult::SetSelfObject(self_object) => {
                self.self_object = Some(self_object.as_ref().clone());
                Ok(())
            }
            _ => Err(RuntimeError::DetailedError(
                "NativeFunctionGenerator received unexpected step result"
                    .to_string()
                    .into(),
            )),
        }
    }

    fn copy(&self) -> Box<dyn Runnable> {
        Box::new(NativeFunctionGenerator {
            argument: self.argument.clone(),
            self_object: self.self_object.clone(),
            function: self.function,
        })
    }

    fn format_context(&self) -> Result<serde_json::Value, RuntimeError> {
        Ok(serde_json::json!({
            "type": "NativeFunctionGenerator",
            "argument": self.argument.to_string(),
        }))
    }
}

pub fn wrap_native_function<F>(
    params: &OnionStaticObject,
    capture: Option<&OnionStaticObject>,
    self_object: Option<&OnionStaticObject>,
    signature: String,
    function: &'static F,
) -> OnionStaticObject
where
    F: Fn(&OnionStaticObject, &mut GC<OnionObjectCell>) -> Result<OnionStaticObject, RuntimeError>
        + Send
        + Sync
        + 'static,
{
    OnionLambdaDefinition::new_static(
        params,
        LambdaBody::NativeFunction(Box::new(NativeFunctionGenerator {
            argument: onion_tuple!(),
            self_object: self_object.cloned(),
            function: function,
        })),
        capture,
        self_object,
        signature,
    )
}

use std::pin::Pin;
use std::task::{Context, Poll};
use std::{
    future::Future,
    task::{RawWaker, RawWakerVTable, Waker},
};

// Assuming LambdaBody has been extended with LambdaBody::AsyncNativeFunction(Box<dyn Runnable>)

// A Runnable that wraps and polls an async Future for a native method,
// returning StepResult::Error(RuntimeError::Pending) when the Future is pending.
pub struct AsyncNativeMethodGenerator<F, Fut>
where
    F: Fn(Option<&OnionStaticObject>, &OnionStaticObject, &mut GC<OnionObjectCell>) -> Fut
        + Send
        + Sync
        + 'static,
    Fut: Future<Output = Result<OnionStaticObject, RuntimeError>> + Send + Sync + 'static,
{
    argument: OnionStaticObject,
    self_object: Option<OnionStaticObject>,
    function: &'static F,
    future: Option<Pin<Box<Fut>>>,
    waker: std::task::Waker,
}

impl<F, Fut> Runnable for AsyncNativeMethodGenerator<F, Fut>
where
    F: Fn(Option<&OnionStaticObject>, &OnionStaticObject, &mut GC<OnionObjectCell>) -> Fut
        + Send
        + Sync
        + 'static,
    Fut: Future<Output = Result<OnionStaticObject, RuntimeError>> + Send + Sync + 'static,
{
    fn step(&mut self, gc: &mut GC<OnionObjectCell>) -> StepResult {
        if self.future.is_none() {
            // Pin the future to the stack and store it
            self.future = Some(Box::pin((self.function)(
                self.self_object.as_ref(),
                &self.argument,
                gc,
            )));
        }

        let future = self.future.as_mut().unwrap();

        let mut context = Context::from_waker(&self.waker);

        match future.as_mut().poll(&mut context) {
            Poll::Ready(result) => {
                self.future = None;
                match result {
                    Ok(obj) => StepResult::Return(obj.into()),
                    Err(e) => StepResult::Error(e),
                }
            }
            Poll::Pending => StepResult::Error(RuntimeError::Pending),
        }
    }

    fn receive(
        &mut self,
        step_result: &StepResult,
        _gc: &mut GC<OnionObjectCell>,
    ) -> Result<(), RuntimeError> {
        match step_result {
            StepResult::Return(result) => {
                self.argument = result.as_ref().clone();
                Ok(())
            }
            StepResult::SetSelfObject(self_object) => {
                self.self_object = Some(self_object.as_ref().clone());
                Ok(())
            }
            _ => Err(RuntimeError::DetailedError(
                "AsyncNativeMethodGenerator received unexpected step result"
                    .to_string()
                    .into(),
            )),
        }
    }

    fn copy(&self) -> Box<dyn Runnable> {
        Box::new(AsyncNativeMethodGenerator {
            argument: self.argument.clone(),
            self_object: self.self_object.clone(),
            function: self.function,
            future: None, // Cannot clone the future, so start fresh
            waker: dummy_waker(),
        })
    }

    fn format_context(&self) -> Result<serde_json::Value, RuntimeError> {
        Ok(serde_json::json!({
            "type": "AsyncNativeMethodGenerator",
            "argument": self.argument.to_string(),
            "future_state": if self.future.is_some() { "active" } else { "idle" },
        }))
    }
}

// 创建一个静态的、无操作的 VTable。
// 这样可以避免在每次创建 Waker 时都构建一个新的 VTable。
const DUMMY_WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(|_| DUMMY_RAW_WAKER, |_| {}, |_| {}, |_| {});

// 创建一个静态的 RawWaker 实例。
const DUMMY_RAW_WAKER: RawWaker = RawWaker::new(std::ptr::null(), &DUMMY_WAKER_VTABLE);

// 一个辅助函数，用于安全地创建一个 Waker。
fn dummy_waker() -> Waker {
    // unsafe: DUMMY_RAW_WAKER 是一个有效的、虽然是无操作的 RawWaker。
    // 它的生命周期是 'static，所以这里是安全的。
    unsafe { Waker::from_raw(DUMMY_RAW_WAKER) }
}

// The wrap_async_native_function
pub fn wrap_async_native_function<F, Fut>(
    params: &OnionStaticObject,
    capture: Option<&OnionStaticObject>,
    self_object: Option<&OnionStaticObject>,
    signature: String,
    function: &'static F,
) -> OnionStaticObject
where
    F: Fn(Option<&OnionStaticObject>, &OnionStaticObject, &mut GC<OnionObjectCell>) -> Fut
        + Send
        + Sync
        + 'static,
    Fut: Future<Output = Result<OnionStaticObject, RuntimeError>> + Send + Sync + 'static,
{
    OnionLambdaDefinition::new_static(
        params,
        LambdaBody::NativeFunction(Box::new(AsyncNativeMethodGenerator {
            argument: onion_tuple!(),
            self_object: self_object.cloned(),
            function: function,
            future: None,
            waker: dummy_waker(),
        })),
        capture,
        self_object,
        signature,
    )
}

pub struct NativeMethodGenerator<F>
where
    F: Fn(
            Option<&OnionStaticObject>,
            &OnionStaticObject,
            &mut GC<OnionObjectCell>,
        ) -> Result<OnionStaticObject, RuntimeError>
        + 'static,
{
    argument: OnionStaticObject,
    self_object: Option<OnionStaticObject>,
    function: &'static F,
}

impl<F> Runnable for NativeMethodGenerator<F>
where
    F: Fn(
            Option<&OnionStaticObject>,
            &OnionStaticObject,
            &mut GC<OnionObjectCell>,
        ) -> Result<OnionStaticObject, RuntimeError>
        + Send
        + Sync
        + 'static,
{
    fn step(&mut self, gc: &mut GC<OnionObjectCell>) -> StepResult {
        unwrap_step_result!(
            (self.function)(self.self_object.as_ref(), &self.argument, gc)
                .map(|result| StepResult::Return(result.into()))
        )
    }

    fn receive(
        &mut self,
        step_result: &StepResult,
        _gc: &mut GC<OnionObjectCell>,
    ) -> Result<(), RuntimeError> {
        match step_result {
            StepResult::Return(result) => {
                self.argument = result.as_ref().clone();
                Ok(())
            }
            StepResult::SetSelfObject(self_object) => {
                self.self_object = Some(self_object.as_ref().clone());
                Ok(())
            }
            _ => Err(RuntimeError::DetailedError(
                "NativeFunctionGenerator received unexpected step result"
                    .to_string()
                    .into(),
            )),
        }
    }

    fn copy(&self) -> Box<dyn Runnable> {
        Box::new(NativeMethodGenerator {
            argument: self.argument.clone(),
            self_object: self.self_object.clone(),
            function: self.function,
        })
    }

    fn format_context(&self) -> Result<serde_json::Value, RuntimeError> {
        Ok(serde_json::json!({
            "type": "NativeMethodGenerator",
            "argument": self.argument.to_string(),
        }))
    }
}

pub fn wrap_native_method_function<F>(
    params: &OnionStaticObject,
    capture: Option<&OnionStaticObject>,
    self_object: Option<&OnionStaticObject>,
    signature: String,
    function: &'static F,
) -> OnionStaticObject
where
    F: Fn(
            Option<&OnionStaticObject>,
            &OnionStaticObject,
            &mut GC<OnionObjectCell>,
        ) -> Result<OnionStaticObject, RuntimeError>
        + Send
        + Sync
        + 'static,
{
    OnionLambdaDefinition::new_static(
        params,
        LambdaBody::NativeFunction(Box::new(NativeMethodGenerator {
            argument: onion_tuple!(),
            self_object: self_object.cloned(),
            function: function,
        })),
        capture,
        self_object,
        signature,
    )
}

pub fn build_module() -> OnionStaticObject {
    let mut module = IndexMap::new();
    module.insert("bytes".to_string(), bytes::build_module());
    module.insert("types".to_string(), types::build_module());
    module.insert("math".to_string(), math::build_module());
    module.insert("string".to_string(), string::build_module());
    module.insert("time".to_string(), time::build_module());
    build_named_dict(module)
}
