import os
import sys
import subprocess
import platform
import shutil
from pathlib import Path


def run_command(cmd, cwd=None):
    """执行命令并实时打印输出"""
    print(f"执行命令: {' '.join(cmd)}")
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"

    process = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        cwd=cwd,
        env=env,
    )

    # 实时打印输出
    for line in iter(process.stdout.readline, ""):
        print(line.rstrip())

    process.stdout.close()
    return_code = process.wait()

    if return_code != 0:
        print(f"命令执行失败，返回值: {return_code}")
        sys.exit(return_code)
    return return_code


def activate_venv():
    """根据操作系统激活虚拟环境"""
    venv_path = Path(".venv")
    if not venv_path.exists():
        print(f"错误: 虚拟环境 {venv_path} 不存在")
        print("请先创建虚拟环境: python -m venv .venv")
        sys.exit(1)

    # 设置虚拟环境路径
    if platform.system() == "Windows":
        python_cmd = str(venv_path / "Scripts" / "python.exe")
    else:
        python_cmd = str(venv_path / "bin" / "python")

    if not os.path.exists(python_cmd):
        print(f"错误: 找不到虚拟环境中的 Python: {python_cmd}")
        sys.exit(1)

    return python_cmd


def build_project(python_cmd):
    """使用 maturin 构建 Rust 项目"""
    print("=== 开始构建项目 ===")

    # 确保 maturin 已安装
    print("检查 maturin 是否安装...")
    try:
        run_command([python_cmd, "-m", "pip", "install", "--upgrade", "maturin"])
    except Exception as e:
        print(f"安装 maturin 失败: {e}")
        sys.exit(1)

    # 获取 maturin 可执行文件路径
    if platform.system() == "Windows":
        maturin_cmd = os.path.join(os.path.dirname(python_cmd), "maturin.exe")
    else:
        maturin_cmd = os.path.join(os.path.dirname(python_cmd), "maturin")

    if not os.path.exists(maturin_cmd):
        print(f"错误: 找不到 maturin: {maturin_cmd}")
        sys.exit(1)

    # 使用 maturin develop 构建并安装到当前环境
    print("使用 maturin 构建并安装项目...")
    run_command([maturin_cmd, "develop", "--release"])

    print("项目已成功构建并安装到 Python 环境")
    return True


def run_tests(python_cmd):
    """运行 Python 测试"""
    print("\n=== 开始运行测试 ===")
    test_file = os.path.join("test", "test_onion.py")

    # 设置环境变量，确保编码正确
    env = os.environ.copy()
    env["PYTHONIOENCODING"] = "utf-8"

    # 运行测试
    process = subprocess.Popen(
        [python_cmd, test_file],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        env=env,
    )

    # 实时打印输出
    for line in iter(process.stdout.readline, ""):
        print(line.rstrip())

    process.stdout.close()
    return_code = process.wait()

    if return_code != 0:
        print(f"测试失败，返回值: {return_code}")
        sys.exit(0)

    print("测试成功完成！")


def main():
    print("===== onion-py 项目构建与测试 =====")

    # 激活虚拟环境
    python_cmd = activate_venv()
    print(f"使用 Python: {python_cmd}")

    # 使用 maturin 构建项目
    build_project(python_cmd)

    # 运行测试
    run_tests(python_cmd)
    print("\n===== 所有步骤已完成 =====")


if __name__ == "__main__":
    main()
