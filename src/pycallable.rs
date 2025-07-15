use std::sync::Arc;

use onion_vm::{
    lambda::runnable::{Runnable, RuntimeError, StepResult},
    onion_tuple,
    types::{
        lambda::definition::{LambdaBody, OnionLambdaDefinition},
        object::{OnionObjectCell, OnionStaticObject},
        tuple::OnionTuple,
    },
    unwrap_step_result, GC,
};
use pyo3::{PyObject, PyResult, Python};

use crate::{
    py_object_to_onion_object, pyerr_to_runtime_error, script::stdlib::dummy_waker, PyOnionObject,
};

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

use pyo3_async_runtimes::tokio::into_future; // 导入 into_future

pub struct PyFunctionGenerator {
    argument: OnionStaticObject,
    self_object: Option<OnionStaticObject>,
    function: Arc<PyObject>,
}

impl Runnable for PyFunctionGenerator {
    fn step(&mut self, _: &mut GC<OnionObjectCell>) -> StepResult {
        Python::with_gil(|py| {
            let function = self.function.clone();
            let argument = PyOnionObject::from_rust(self.argument.clone());
            let self_object = self
                .self_object
                .clone()
                .map(|obj| PyOnionObject::from_rust(obj));

            // Call the Python function with the provided arguments
            let result = function.call1(py, (self_object, argument));

            // 检查result是否为PyOnionObject
            if !result.is_ok() {
                return StepResult::Error(pyerr_to_runtime_error(result.unwrap_err(), py));
            }

            let result = result.unwrap();
            // Convert the result back to OnionStaticObject
            let result =
                unwrap_step_result!(py_object_to_onion_object(py, result)
                    .map_err(|e| pyerr_to_runtime_error(e, py)));
            StepResult::Return(result.into())
        })
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
        Box::new(PyFunctionGenerator {
            argument: self.argument.clone(),
            self_object: self.self_object.clone(),
            function: self.function.clone(),
        })
    }

    fn format_context(&self) -> Result<serde_json::Value, RuntimeError> {
        Ok(serde_json::json!({
            "type": "NativeFunctionGenerator",
            "argument": self.argument.to_string(),
        }))
    }
}

pub fn wrap_py_function(
    params: &OnionStaticObject,
    capture: Option<&OnionStaticObject>,
    self_object: Option<&OnionStaticObject>,
    signature: String,
    function: PyObject,
) -> OnionStaticObject {
    OnionLambdaDefinition::new_static(
        params,
        LambdaBody::NativeFunction(Box::new(PyFunctionGenerator {
            argument: onion_tuple!(),
            self_object: self_object.cloned(),
            function: Arc::new(function),
        })),
        capture,
        self_object,
        signature,
    )
}

pub struct PyCoroutineGenerator {
    // 存储原始的 Python 协程对象
    python_coroutine: PyObject,
    // 存储转换为 Rust Future 后的对象
    rust_future: Option<Pin<Box<dyn Future<Output = PyResult<PyObject>> + Send + Sync + 'static>>>,
    // 参数和 self 绑定，通过 receive 方法设置
    argument: OnionStaticObject,
    self_object: Option<OnionStaticObject>,
    // 需要一个 Waker，可以使用 AsyncNativeMethodGenerator 中的 dummy_waker
    waker: Waker,
}

impl Runnable for PyCoroutineGenerator {
    fn step(&mut self, _: &mut GC<OnionObjectCell>) -> StepResult {
        // 确保在与 Python 交互时持有 GIL
        Python::with_gil(|py| {
            // 如果还没有转换为 Rust Future，则进行转换
            if self.rust_future.is_none() {
                let coroutine_obj = match self.python_coroutine.call1(
                    py,
                    (
                        self.self_object
                            .as_ref()
                            .cloned()
                            .map(PyOnionObject::from_rust),
                        PyOnionObject::from_rust(self.argument.clone()),
                    ),
                ) {
                    Ok(obj) => obj,
                    Err(e) => return StepResult::Error(pyerr_to_runtime_error(e, py)),
                };
                // 将 Python 协程转换为 Rust Future
                let rust_fut_result = into_future(coroutine_obj.into_bound(py));

                match rust_fut_result {
                    Ok(fut) => {
                        // 存储转换后的 Rust Future
                        self.rust_future = Some(Box::pin(fut));
                    }
                    Err(e) => {
                        // 转换失败，返回错误
                        return StepResult::Error(pyerr_to_runtime_error(e, py));
                    }
                }
            }

            // Poll 存储的 Rust Future
            let future = self.rust_future.as_mut().unwrap();
            let mut context = Context::from_waker(&self.waker); // 使用 dummy waker

            match future.as_mut().poll(&mut context) {
                Poll::Ready(py_result) => {
                    // Future 完成，处理结果
                    self.rust_future = None; // Future 已完成，可以丢弃

                    match py_result {
                        Ok(py_obj) => {
                            // Python 函数成功返回，将结果转换回 OnionStaticObject
                            match py_object_to_onion_object(py, py_obj.into()) {
                                Ok(onion_obj) => StepResult::Return(onion_obj.into()),
                                Err(e) => {
                                    // 转换回 OnionStaticObject 失败
                                    return StepResult::Error(pyerr_to_runtime_error(e, py));
                                }
                            }
                        }
                        Err(py_err) => {
                            // Python 函数抛出异常
                            StepResult::Error(pyerr_to_runtime_error(py_err, py))
                        }
                    }
                }
                Poll::Pending => {
                    // Future 仍在等待，返回 Pending
                    StepResult::Error(RuntimeError::Pending)
                }
            }
        }) // GIL 释放
    }

    fn receive(
        &mut self,
        step_result: &StepResult,
        _gc: &mut GC<OnionObjectCell>,
    ) -> Result<(), RuntimeError> {
        // 实现参数和 self 绑定的接收逻辑，类似于 NativeFunctionGenerator
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
                "PythonCoroutineGenerator received unexpected step result"
                    .to_string()
                    .into(),
            )),
        }
    }

    fn copy(&self) -> Box<dyn Runnable> {
        // 实现 copy 方法
        let python_coroutine = Python::with_gil(|py| self.python_coroutine.clone_ref(py));
        Box::new(PyCoroutineGenerator {
            // 克隆 Python 对象引用
            python_coroutine,
            // Future 不能克隆，所以在拷贝中设置为 None
            rust_future: None,
            // 克隆参数和 self 绑定
            argument: self.argument.clone(),
            self_object: self.self_object.clone(),
            // 使用 dummy waker
            waker: dummy_waker(),
        })
    }

    fn format_context(&self) -> Result<serde_json::Value, RuntimeError> {
        // 实现 format_context
        Ok(serde_json::json!({
            "type": "PythonCoroutineGenerator",
            "future_state": if self.rust_future.is_some() { "active" } else { "idle" },
            // 可以添加更多上下文信息，例如参数和 self_object 的表示
        }))
    }
}

pub fn wrap_py_coroutine(
    params: &OnionStaticObject,
    capture: Option<&OnionStaticObject>,
    self_object: Option<&OnionStaticObject>,
    signature: String,
    function: PyObject,
) -> OnionStaticObject {
    OnionLambdaDefinition::new_static(
        params,
        LambdaBody::NativeFunction(Box::new(PyCoroutineGenerator {
            python_coroutine: function,
            argument: onion_tuple!(),
            self_object: self_object.cloned(),
            rust_future: None,
            waker: dummy_waker(),
        })),
        capture,
        self_object,
        signature,
    )
}
