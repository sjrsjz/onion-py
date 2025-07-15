use arc_gc::arc::GCArcWeak;
use arc_gc::traceable::GCTraceable;
use onion_frontend::dir_stack::DirectoryStack;
use onion_vm::lambda::runnable::RuntimeError;
use onion_vm::types::named::OnionNamed;
use onion_vm::types::object::{OnionObject, OnionObjectCell, OnionObjectExt, OnionStaticObject};
use onion_vm::types::pair::OnionPair;
// 引入 RuntimeError
use onion_vm::types::tuple::OnionTuple;
use pyo3::exceptions::PyTypeError; // 引入 PyTypeError
use pyo3::types::PyAny;
use pyo3::{prelude::*, IntoPyObjectExt};
use pyo3_async_runtimes::tokio::future_into_py;
use std::fmt::Debug;
use std::sync::Arc;

mod pycallable;
mod script;

// Helper function to convert RuntimeError to PyErr
fn runtime_error_to_pyerr(err: RuntimeError) -> PyErr {
    PyTypeError::new_err(err.to_string()) // 将 Runtime Error 转换为 Python 的 TypeError
}

fn pyerr_to_runtime_error(e: PyErr, py: Python<'_>) -> RuntimeError {
    return RuntimeError::CustomValue(
        OnionObject::Custom(Arc::new(OnionPyObject {
            inner: match e.into_py_any(py) {
                Ok(obj) => obj,
                Err(e) => {
                    return RuntimeError::DetailedError(
                        format!("Failed to call Python coroutine: {}", e).into(),
                    );
                }
            },
        }))
        .stabilize()
        .into(),
    );
}

// 定义 Python 包装类
#[pyclass]
#[derive(Clone)] // 允许在 Python 中克隆对象
pub struct PyOnionObject {
    inner: OnionStaticObject,
}

#[pymethods]
impl PyOnionObject {
    #[new]
    fn new(obj: PyObject, py: Python) -> PyResult<Self> {
        let onion_obj = py_object_to_onion_object(py, obj)?;
        Ok(Self::from_rust(onion_obj))
    }
    // --- 类型检查方法 ---
    fn is_integer(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Integer(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_float(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Float(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_string(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::String(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_bytes(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Bytes(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_boolean(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Boolean(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_null(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Null)))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_undefined(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Undefined(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_range(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Range(_, _))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_tuple(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Tuple(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_pair(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Pair(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_named(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Named(_))))
            .map_err(runtime_error_to_pyerr)
    }

    fn is_custom(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .with_data(|obj| Ok(matches!(obj, OnionObject::Custom(_))))
            .map_err(runtime_error_to_pyerr)
    }

    // --- 值获取方法（带类型转换）---
    fn as_integer(&self) -> PyResult<i64> {
        self.inner
            .weak()
            .to_integer()
            .map_err(runtime_error_to_pyerr) // 转换 RuntimeError 为 PyErr
    }

    fn as_float(&self) -> PyResult<f64> {
        self.inner.weak().to_float().map_err(runtime_error_to_pyerr)
    }

    fn as_string(&self) -> PyResult<String> {
        // to_string 方法需要一个 ptrs 参数，这里传递一个空 Vec
        self.inner
            .weak()
            .to_string(&vec![])
            .map_err(runtime_error_to_pyerr)
    }

    fn as_bytes(&self) -> PyResult<Vec<u8>> {
        self.inner.weak().to_bytes().map_err(runtime_error_to_pyerr)
    }

    fn as_boolean(&self) -> PyResult<bool> {
        self.inner
            .weak()
            .to_boolean()
            .map_err(runtime_error_to_pyerr)
    }
    fn as_range(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .with_data(|obj| {
                match obj {
                    OnionObject::Range(start, end) => {
                        // 创建一个新的 OnionStaticObject::Range 并包装
                        let range_obj = OnionObject::Range(*start, *end).stabilize();
                        Ok(PyOnionObject::from_rust(range_obj))
                    }
                    _ => Err(RuntimeError::InvalidType(
                        format!("Object is not a Range: {:?}", obj).into(),
                    )
                    .into()),
                }
            })
            .map_err(runtime_error_to_pyerr)
    }

    fn as_tuple(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .with_data(|obj| {
                match obj {
                    OnionObject::Tuple(tuple) => {
                        // 克隆 Tuple 并包装
                        let tuple_obj = OnionObject::Tuple(tuple.clone()).stabilize();
                        Ok(PyOnionObject::from_rust(tuple_obj))
                    }
                    _ => Err(RuntimeError::InvalidType(
                        format!("Object is not a Tuple: {:?}", obj).into(),
                    )
                    .into()),
                }
            })
            .map_err(runtime_error_to_pyerr)
    }

    fn as_pair(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .with_data(|obj| {
                match obj {
                    OnionObject::Pair(pair) => {
                        // 克隆 Pair 并包装
                        let pair_obj = OnionObject::Pair(pair.clone()).stabilize();
                        Ok(PyOnionObject::from_rust(pair_obj))
                    }
                    _ => Err(RuntimeError::InvalidType(
                        format!("Object is not a Pair: {:?}", obj).into(),
                    )
                    .into()),
                }
            })
            .map_err(runtime_error_to_pyerr)
    }

    fn as_named(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .with_data(|obj| {
                match obj {
                    OnionObject::Named(name) => {
                        // 克隆 Named 并包装
                        let named_obj = OnionObject::Named(name.clone()).stabilize();
                        Ok(PyOnionObject::from_rust(named_obj))
                    }
                    _ => Err(RuntimeError::InvalidType(
                        format!("Object is not a Named: {:?}", obj).into(),
                    )
                    .into()),
                }
            })
            .map_err(runtime_error_to_pyerr)
    }

    // --- 核心操作方法 ---
    fn type_name(&self) -> PyResult<String> {
        self.inner.weak().type_of().map_err(runtime_error_to_pyerr)
    }

    // 实现 Python 的 __repr__ 和 __str__
    fn __repr__(&self) -> PyResult<String> {
        // repr 方法需要一个 ptrs 参数，这里传递一个空 Vec
        self.inner
            .weak()
            .repr(&vec![])
            .map_err(runtime_error_to_pyerr)
    }

    fn __str__(&self) -> PyResult<String> {
        // to_string 方法需要一个 ptrs 参数，这里传递一个空 Vec
        self.inner
            .weak()
            .to_string(&vec![])
            .map_err(runtime_error_to_pyerr)
    }

    fn len(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .len()
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn key(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .key_of()
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn value(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .value_of()
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __len__(&self) -> PyResult<usize> {
        self.inner
            .weak()
            .len()
            .map_err(runtime_error_to_pyerr)?
            .weak()
            .to_integer()
            .map(|len| len as usize)
            .map_err(runtime_error_to_pyerr)
    }

    // Implement Python's __contains__
    fn __contains__(&self, item: PyObject, py: Python) -> PyResult<bool> {
        let onion_item = py_object_to_onion_object(py, item)?;
        self.inner
            .weak()
            .contains(onion_item.weak())
            .map_err(runtime_error_to_pyerr)
    }

    // Implement Python's __getitem__ for indexing
    fn __getitem__(&self, index: PyObject, py: Python) -> PyResult<Self> {
        let index_i64: i64 = index.extract(py)?; // Assuming integer index
        self.inner
            .weak()
            .at(index_i64)
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __getattr__(&self, attr: String, _py: Python) -> PyResult<Self> {
        self.inner
            .weak()
            .with_attribute(&OnionObject::String(attr.into()), &|obj| {
                Ok(Self::from_rust(obj.stabilize()))
            })
            .map_err(runtime_error_to_pyerr)
    }

    fn __setattr__(&self, attr: String, _value: PyObject, _py: Python) -> PyResult<()> {
        // 由于OnionVM的对象的严格不可变性，无法修改属性
        Err(PyTypeError::new_err(format!(
            "Cannot set attribute {} on PyOnionObject as it is immutable",
            attr
        )))
    }

    fn __eq__(&self, other: PyObject, py: Python) -> PyResult<bool> {
        if let Ok(other_onion) = other.extract::<PyRef<PyOnionObject>>(py) {
            self.inner
                .weak()
                .equals(other_onion.inner.weak())
                .map_err(runtime_error_to_pyerr)
        } else {
            Err(PyTypeError::new_err(format!(
                "Cannot compare PyOnionObject with type {:?}",
                other
            )))
        }
    }

    fn __lt__(&self, other: PyObject, py: Python) -> PyResult<bool> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_lt(onion_other.weak())
            .map_err(runtime_error_to_pyerr)
    }

    fn __gt__(&self, other: PyObject, py: Python) -> PyResult<bool> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_gt(onion_other.weak())
            .map_err(runtime_error_to_pyerr)
    }

    fn __add__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_add(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __sub__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_sub(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __mul__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_mul(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __truediv__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_div(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __mod__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_mod(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __pow__(&self, other: PyObject, modulo: Option<PyObject>, py: Python) -> PyResult<Self> {
        if modulo.is_some() {
            // The underlying binary_pow does not support the three-argument form of pow
            Err(PyTypeError::new_err(
                "Three-argument pow() is not supported for PyOnionObject",
            ))
        } else {
            let onion_other = py_object_to_onion_object(py, other)?;
            self.inner
                .weak()
                .binary_pow(onion_other.weak())
                .map(Self::from_rust)
                .map_err(runtime_error_to_pyerr)
        }
    }

    fn __and__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_and(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __or__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_or(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __xor__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_xor(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __lshift__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_shl(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn __rshift__(&self, other: PyObject, py: Python) -> PyResult<Self> {
        let onion_other = py_object_to_onion_object(py, other)?;
        self.inner
            .weak()
            .binary_shr(onion_other.weak())
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    // Implement Python's __neg__
    fn __neg__(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .unary_neg()
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    // Implement Python's __pos__
    fn __pos__(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .unary_plus()
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    // Implement Python's __invert__ (assuming unary_not is bitwise NOT)
    fn __invert__(&self) -> PyResult<Self> {
        self.inner
            .weak()
            .unary_not() // Assuming unary_not maps to bitwise NOT
            .map(Self::from_rust)
            .map_err(runtime_error_to_pyerr)
    }

    fn unwrap_py(&self, py: Python) -> PyResult<PyObject> {
        // 将 OnionObject::Custom 转换为 PyOnionObject
        match self.inner.weak() {
            OnionObject::Custom(custom) => {
                match custom.as_any().downcast_ref::<OnionPyObject>() {
                    // 如果是 PyOnionObject，返回其 PyObject
                    Some(py_onion) => Ok(py_onion.inner.clone_ref(py)),
                    None => {
                        // 否则返回错误
                        Err(PyTypeError::new_err(
                            "Cannot unwrap non-PyOnionObject custom OnionObject",
                        ))
                    }
                }
            }
            _ => Err(PyTypeError::new_err("Cannot unwrap non-custom OnionObject")),
        }
    }

    #[staticmethod]
    fn pair(k: PyObject, v: PyObject, py: Python) -> PyResult<Self> {
        let k = py_object_to_onion_object(py, k)?;
        let v = py_object_to_onion_object(py, v)?;
        Ok(Self::from_rust(OnionPair::new_static(&k, &v)))
    }

    #[staticmethod]
    fn named(k: PyObject, v: PyObject, py: Python) -> PyResult<Self> {
        let k = py_object_to_onion_object(py, k)?;
        let v = py_object_to_onion_object(py, v)?;
        Ok(Self::from_rust(OnionNamed::new_static(&k, &v)))
    }

    #[staticmethod]
    fn tuple(elements: PyObject, py: Python) -> PyResult<Self> {
        let tuple = py_object_to_onion_object(py, elements)?;
        Ok(Self::from_rust(tuple))
    }
}

impl PyOnionObject {
    // 内部使用的工厂方法，从 Rust 的 OnionStaticObject 创建 PyOnionObject
    fn from_rust(obj: OnionStaticObject) -> Self {
        PyOnionObject { inner: obj }
    }
}

pub struct OnionPyObject {
    inner: PyObject,
}

impl Debug for OnionPyObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OnionPyObject({:?})", self.inner)
    }
}

impl GCTraceable<OnionObjectCell> for OnionPyObject {
    fn collect(&self, _: &mut std::collections::VecDeque<GCArcWeak<OnionObjectCell>>) {}
}

impl OnionObjectExt for OnionPyObject {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn upgrade(&self, _: &mut Vec<arc_gc::arc::GCArc<OnionObjectCell>>) {
        // nothing
    }

    fn equals(&self, _: &OnionObject) -> Result<bool, RuntimeError> {
        Ok(false)
    }

    fn is_same(&self, _: &OnionObject) -> Result<bool, RuntimeError> {
        Ok(false)
    }

    fn type_of(&self) -> Result<String, RuntimeError> {
        Ok("PythonObject".to_string())
    }

    fn repr(&self, _: &Vec<*const OnionObject>) -> Result<String, RuntimeError> {
        // 使用 Python 的 __repr__ 方法
        Python::with_gil(|py| match self.inner.call_method0(py, "__repr__") {
            Ok(result) => result
                .extract(py)
                .map_err(|e| pyerr_to_runtime_error(e, py)),
            Err(e) => Err(pyerr_to_runtime_error(e, py)),
        })
    }

    fn to_string(&self, _: &Vec<*const OnionObject>) -> Result<String, RuntimeError> {
        // 使用 Python 的 __str__ 方法
        Python::with_gil(|py| match self.inner.call_method0(py, "__str__") {
            Ok(result) => result
                .extract(py)
                .map_err(|e| pyerr_to_runtime_error(e, py)),
            Err(e) => Err(pyerr_to_runtime_error(e, py)),
        })
    }
}

// Helper function to convert OnionObject basic types to Python objects
// 修改此函数以返回 PyOnionObject 实例
pub fn onion_object_to_py(py: Python<'_>, obj: &OnionObject) -> PyResult<PyObject> {
    // 将 OnionObject 转换为 OnionStaticObject
    let static_obj = obj.stabilize();
    // 创建 PyOnionObject 实例并返回其 PyObject 表示
    PyOnionObject::from_rust(static_obj).into_py_any(py)
}

// Helper function to convert Python objects to OnionObject basic types
pub fn py_object_to_onion_object(py: Python<'_>, obj: Py<PyAny>) -> PyResult<OnionStaticObject> {
    // 检查输入是否是 PyOnionObject 的实例
    if let Ok(py_onion) = obj.extract::<PyRef<PyOnionObject>>(py) {
        // 如果是, 返回其内部的 OnionStaticObject
        Ok(py_onion.inner.clone()) // 需要克隆，因为返回的是 OnionStaticObject
    } else if let Ok(i) = obj.extract::<i64>(py) {
        Ok(OnionObject::Integer(i).stabilize())
    } else if let Ok(f) = obj.extract::<f64>(py) {
        Ok(OnionObject::Float(f).stabilize())
    } else if let Ok(s) = obj.extract::<String>(py) {
        Ok(OnionObject::String(Arc::new(s)).stabilize())
    } else if let Ok(b) = obj.extract::<bool>(py) {
        Ok(OnionObject::Boolean(b).stabilize())
    } else if obj.is_none(py) {
        Ok(OnionObject::Null.stabilize())
    } else if let Ok(bytes) = obj.downcast_bound::<pyo3::types::PyBytes>(py) {
        // Explicitly handle Python bytes objects
        Ok(OnionObject::Bytes(Arc::new(bytes.as_bytes().to_vec())).stabilize())
    } else if let Ok(tuple) = obj.downcast_bound::<pyo3::types::PyTuple>(py) {
        // Convert Python tuple to OnionObject::Tuple
        let mut elements = Vec::new();
        for item in tuple.iter() {
            // Recursively convert tuple elements
            elements.push(py_object_to_onion_object(py, item.into())?);
        }
        // OnionTuple::new_static_no_ref 需要 OnionStaticObject 的 Vec
        let onion_tuple_elements: Vec<OnionStaticObject> = elements.into_iter().collect();
        Ok(OnionTuple::new_static_no_ref(&onion_tuple_elements))
    } else if let Ok(list) = obj.downcast_bound::<pyo3::types::PyList>(py) {
        // Convert Python list to OnionObject::List
        let mut elements = Vec::new();
        for item in list.iter() {
            // Recursively convert list elements
            elements.push(py_object_to_onion_object(py, item.into())?);
        }
        // OnionTuple::new_static_no_ref 需要 OnionStaticObject 的 Vec
        let onion_tuple_elements: Vec<OnionStaticObject> = elements.into_iter().collect();
        Ok(OnionTuple::new_static_no_ref(&onion_tuple_elements))
    } else if let Ok(set) = obj.downcast_bound::<pyo3::types::PySet>(py) {
        // Convert Python set to OnionObject::Set
        let mut elements = Vec::new();
        for item in set.iter() {
            // Recursively convert set elements
            elements.push(py_object_to_onion_object(py, item.into())?);
        }
        // OnionTuple::new_static_no_ref 需要 OnionStaticObject 的 Vec
        let onion_tuple_elements: Vec<OnionStaticObject> = elements.into_iter().collect();
        Ok(OnionTuple::new_static_no_ref(&onion_tuple_elements))
    } else {
        Ok(OnionObject::Custom(Arc::new(OnionPyObject { inner: obj.into() })).stabilize())
    }
}

/// An asynchronous Python function implemented in Rust.
#[pyfunction]
fn eval<'pya>(
    py: Python<'pya>,
    code: String,
    work_dir: Option<String>,
    context: Option<PyObject>,
) -> PyResult<Bound<'pya, PyAny>> {
    // Extract context to a serializable form before entering async block
    let context_serialized = if let Some(ctx) = context {
        // Extract the context list in the current thread (with GIL)
        let ctx_list: Vec<PyOnionObject> = ctx.extract(py)?;
        let context_variables: Vec<OnionStaticObject> =
            ctx_list.into_iter().map(|obj| obj.inner).collect();
        Some(context_variables)
    } else {
        None
    };

    future_into_py(py, async move {
        let work_dir_pathbuf = work_dir.map(|path| std::path::PathBuf::from(path));
        let mut dir_stack = match DirectoryStack::new(work_dir_pathbuf.as_deref()) {
            Ok(stack) => stack,
            Err(err) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to create directory stack: {}",
                    err
                )));
            }
        };
        let context_variables_ref: Option<Vec<&OnionStaticObject>> =
            context_serialized.as_ref().map(|v| v.iter().collect());
        let result = match script::eval(&code, &mut dir_stack, context_variables_ref).await {
            Ok(value) => value,
            Err(err) => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to evaluate script: {}",
                    err
                )));
            }
        };
        Python::with_gil(|py| PyOnionObject::from_rust(result).into_py_any(py))
    })
}

#[pyfunction]
fn wrap_py_function<'py>(
    params: PyObject,
    signature: String,
    function: PyObject,
    capture: Option<PyObject>,
    self_object: Option<PyObject>,
    py: Python<'py>,
) -> PyResult<PyOnionObject> {
    // Wrap the Python function into an OnionLambdaDefinition
    let params_onion = py_object_to_onion_object(py, params)?;
    let capture_onion = capture
        .map(|c| py_object_to_onion_object(py, c))
        .transpose()?;
    let self_object_onion = self_object
        .map(|s| py_object_to_onion_object(py, s))
        .transpose()?;
    Ok(PyOnionObject::from_rust(pycallable::wrap_py_function(
        &params_onion,
        capture_onion.as_ref(),
        self_object_onion.as_ref(),
        signature,
        function,
    )))
}

#[pyfunction]
fn wrap_py_coroutine<'py>(
    params: PyObject,
    signature: String,
    coroutine: PyObject,
    capture: Option<PyObject>,
    self_object: Option<PyObject>,
    py: Python<'py>,
) -> PyResult<PyOnionObject> {
    // Wrap the Python coroutine into a PythonCoroutineGenerator
    let params_onion = py_object_to_onion_object(py, params)?;
    let capture_onion = capture
        .map(|c| py_object_to_onion_object(py, c))
        .transpose()?;
    let self_object_onion = self_object
        .map(|s| py_object_to_onion_object(py, s))
        .transpose()?;
    Ok(PyOnionObject::from_rust(pycallable::wrap_py_coroutine(
        &params_onion,
        capture_onion.as_ref(),
        self_object_onion.as_ref(),
        signature,
        coroutine,
    )))
}

#[pymodule(name = "onion")]
fn onion_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(eval, m)?)?;
    m.add_function(wrap_pyfunction!(wrap_py_function, m)?)?;
    m.add_function(wrap_pyfunction!(wrap_py_coroutine, m)?)?;
    m.add_class::<PyOnionObject>()?; // 注册新的 Python 类
    Ok(())
}
