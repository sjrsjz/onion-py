import unittest
import asyncio
from typing import Awaitable, Any
import asyncio

from onion import (
    eval,
    PyOnionObject,
    wrap_py_function,
    wrap_py_coroutine,
    eval_or_throw,
    OnionRuntimeError,
)


class TestOnion(unittest.TestCase):
    def setUp(self):
        pass

    def test_eval(self):
        async def test():
            result = await eval(
                """
                return (1..10).elements();
                """,
                None,
                None,
            )
            print(result)
            return result

        asyncio.run(test())

    def test_py_onion_object(self):
        A = PyOnionObject("A")
        print("A:", A)
        B = PyOnionObject("B")
        print("B:", B)
        self.assertTrue(A.is_string())
        self.assertFalse(B.is_integer())
        pair = PyOnionObject.pair(A, B)
        print("Pair:", pair, "key:", pair.key(), "value:", pair.value())
        self.assertTrue(pair.is_pair())
        named = PyOnionObject.named("test", A)
        print("Named:", named, "key:", named.key(), "value:", named.value())
        tuple = PyOnionObject(
            [
                PyOnionObject("item1"),
                PyOnionObject("item2"),
            ]
        )
        print("Tuple:", tuple)
        self.assertTrue(tuple.is_tuple())

        dict_obj = PyOnionObject(
            [
                PyOnionObject.pair("key1", "Hello"),
                PyOnionObject.pair("key2", "World"),
            ]
        )
        print("Dict:", dict_obj, "value1:", dict_obj.key1, "value2:", dict_obj.key2)

    def test_call_py_function(self):

        def add(self_object: PyOnionObject, arguments: PyOnionObject):
            a = arguments.a.as_integer()
            b = arguments.b.as_integer()
            return a + b

        async def async_add(
            self_object: PyOnionObject, arguments: PyOnionObject
        ) -> Awaitable[Any]:
            a = arguments.a.as_integer()
            b = arguments.b.as_integer()
            await asyncio.sleep(1)
            raise ValueError("模拟异步错误")  # 模拟异步错误

        async def test():
            context = [
                PyOnionObject.named(
                    "add",
                    wrap_py_function(
                        PyOnionObject(
                            [
                                PyOnionObject.named("a", None),
                                PyOnionObject.named("b", None),
                            ]
                        ),
                        "<python>::add",
                        add,
                        None,
                        None,
                    ),
                ),
                PyOnionObject.named(
                    "async_add",
                    wrap_py_coroutine(
                        PyOnionObject(
                            [
                                PyOnionObject.named("a", None),
                                PyOnionObject.named("b", None),
                            ]
                        ),
                        "<python>::async_add",
                        async_add,
                        None,
                        None,
                    ),
                ),
            ]
            print("Context:", context)
            result = await eval(
                """
                @required add;
                return add(3, 5);
                """,
                None,
                context,
            )
            print("Call result:", repr(result))
            self.assertEqual(result.value().as_integer(), 8)

            try:
                result = await eval_or_throw(
                    """
                    @required async_add;
                    return async_add(3, 5);
                    """,
                    None,
                    context,
                )
            except Exception as e:
                print("Caught exception:", e)
                print("Exception type:", type(e))
                print("Exception value:", repr(e.value.unwrap_py()))
                self.assertIsInstance(e, OnionRuntimeError)

        asyncio.run(test())


if __name__ == "__main__":
    unittest.main()
