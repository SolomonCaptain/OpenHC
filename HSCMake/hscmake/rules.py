import os
import subprocess
import sys
from abc import ABC, abstractmethod
from pathlib import Path
from typing import List, Optional
from pex import toml

from .model import Target, Language


class BuildRule(ABC):
    def __init__(self, target: Target, build_dir: Path):
        self.target = target
        self.build_dir = build_dir
        self.generated_files: List[Path] = []
        self.command: Optional[List[str]] = None

    @abstractmethod
    def generate(self):
        """生成必要的构建文件（如 Ninja）"""
        pass

    @abstractmethod
    def build(self):
        """执行实际构建命令"""
        pass


class CppNinjaRule(BuildRule):
    def generate(self):
        target_dir = self.build_dir / self.target.name
        target_dir.mkdir(parents=True, exist_ok=True)
        ninja_file = target_dir / "build.ninja"
        self.generated_files.append(ninja_file)

        cxx = "g++"
        cflags = self.target.compile_options.get("COMPILE_OPTIONS", [])
        if isinstance(cflags, list):
            cflags = " ".join(cflags)
        ldflags = " ".join(self.target.link_options)

        objs = []
        for src in self.target.sources:
            obj = target_dir / (src.path.stem + ".o")
            objs.append(obj)

        with open(ninja_file, "w") as f:
            f.write(f"cxx = {cxx}\n")
            f.write(f"cflags = {cflags}\n")
            f.write(f"ldflags = {ldflags}\n\n")
            f.write("rule cxx\n")
            f.write("  command = $cxx -MMD -MF $out.d $cflags -c $in -o $out\n")
            f.write("  depfile = $out.d\n")
            f.write("  deps = gcc\n\n")
            f.write("rule link\n")
            f.write("  command = $cxx $ldflags -o $out $in\n\n")

            # 使用相对于 target_dir 的路径作为源文件，输出文件仅用文件名
            for src, obj in zip(self.target.sources, objs):
                rel_src = os.path.relpath(src.path.resolve(), start=target_dir)
                rel_src = rel_src.replace('\\', '/')
                obj_name = obj.name
                f.write(f"build {obj_name}: cxx {rel_src}\n")

            output_exe_name = self.target.output_name + (".exe" if sys.platform == "win32" else "")
            objs_names = [obj.name for obj in objs]
            f.write(f"build {output_exe_name}: link {' '.join(objs_names)}\n")
            f.write(f"default {output_exe_name}\n")

    def build(self):
        target_dir = self.build_dir / self.target.name
        ninja_file = target_dir / "build.ninja"
        subprocess.run(["ninja", "-f", str(ninja_file.name)], cwd=target_dir, check=True)


class RustCargoRule(BuildRule):
    def generate(self):
        src_path = self.target.sources[0].path
        cargo_dir = src_path.parent
        if not (cargo_dir / "Cargo.toml").exists():
            raise FileNotFoundError(
                f"No Cargo.toml found in {cargo_dir}. Please provide one."
            )
        self.cargo_dir = cargo_dir

        # 读取 Cargo.toml 来获取实际的包名（必须使用二进制模式）
        cargo_toml_path = cargo_dir / "Cargo.toml"
        with open(cargo_toml_path, 'rb') as f:
            cargo_data = toml.load(f)

        package_name = cargo_data.get('package', {}).get('name', self.target.name)

        self.command = [
            "cargo",
            "build",
            "--manifest-path",
            str(cargo_toml_path),
            "--target-dir",
            str(self.build_dir / "cargo"),
        ]

        # 如果有 [[bin]] 部分，使用实际的二进制名称
        bins = cargo_data.get('bin', [])
        if bins:
            bin_name = bins[0].get('name', package_name)
            self.command.extend(['--bin', bin_name])
        else:
            # 否则使用包名作为二进制名
            self.command.extend(['--bin', package_name])

        profile = self.target.compile_options.get("PROFILE", "debug")
        if profile == "release":
            self.command.append("--release")

    def build(self):
        if not self.command:
            self.generate()
        try:
            subprocess.run(self.command, check=True)
        except FileNotFoundError:
            raise RuntimeError(
                "cargo command not found. Please install Rust from https://www.rust-lang.org/tools/install\n"
                "And ensure cargo is in your PATH"
            )
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"Cargo build failed: {e}")


class TypeScriptTSCRule(BuildRule):
    def generate(self):
        out_dir = self.target.compile_options.get("OUT_DIR", "dist")
        if isinstance(out_dir, list):
            out_dir = out_dir[0] if out_dir else "dist"
        output_path = self.build_dir / self.target.name / out_dir
        output_path.mkdir(parents=True, exist_ok=True)
        src_files = [str(s.path) for s in self.target.sources]
        self.command = ["tsc", "--outDir", str(output_path)]
        extra = self.target.compile_options.get("COMPILE_OPTIONS", [])
        if isinstance(extra, str):
            extra = [extra]
        elif isinstance(extra, list):
            pass
        else:
            extra = []
        self.command.extend(extra)
        self.command.extend(src_files)

    def build(self):
        if not self.command:
            self.generate()
        try:
            subprocess.run(self.command, check=True)
        except FileNotFoundError:
            raise RuntimeError(
                "tsc command not found. Please install TypeScript: npm install -g typescript"
            )
        except subprocess.CalledProcessError as e:
            raise RuntimeError(f"TypeScript compilation failed: {e}")