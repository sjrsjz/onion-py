import unittest
import asyncio

from onion_py import eval, PyOnionObject

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
        tuple = PyOnionObject([
            PyOnionObject("item1"),
            PyOnionObject("item2"),
        ])
        print("Tuple:", tuple)
        self.assertTrue(tuple.is_tuple())

        dict_obj = PyOnionObject([
            PyOnionObject.pair("key1", "Hello"),
            PyOnionObject.pair("key2", "World"),
        ])
        print("Dict:", dict_obj, "value1:", dict_obj.key1, "value2:", dict_obj.key2)

if __name__ == "__main__":
    unittest.main()
