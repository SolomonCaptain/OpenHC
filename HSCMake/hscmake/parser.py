import ast
from pathlib import Path
from typing import Any, Dict, List, Optional

from .model import Project, Target, TargetType, Language, SourceFile

class HSCMakeParser(ast.NodeVisitor):
    def __init__(self):
        self.project = Project(name="")
        self.functions = {
            "project": self._handle_project,
            "add_executable": self._handle_add_executable,
        }
        
    def parse_file(self, filepath: Path) -> Project:
        with open(filepath, "r", encoding="utf-8") as f:
            source = f.read()
        tree = ast.parse(source)
        self.visit(tree)
        return self.project
        
    def visit_Call(self, node: ast.Call):
        if isinstance(node.func, ast.Name):
            func_name = node.func.id
            if func_name in self.functions:
                self.functions[func_name](node)
        self.generic_visit(node)
        
    def _handle_project(self, node: ast.Call):
        kwargs = self._parse_keyword_arguments(node)
        self.project.name = kwargs.get("name", "")
        self.project.version = kwargs.get("VERSION", "0.1.0")
        langs = kwargs.get("LANGUAGES", [])
        for lang in langs:
            if isinstance(lang, str):
                try:
                    self.project.languages.append(Language(lang.lower()))
                except ValueError:
                    pass
                    
    def _handle_add_executable(self, node: ast.Call):
        args = node.args
        if len(args) < 1:
            return
        target_name = self._get_constant_value(args[0])
        kwargs = self._parse_keyword_arguments(node)
        
        # 收集源文件列表
        sources = kwargs.get("SOURCES", [])
        if not sources:
            sources = [
                self._get_constant_value(arg)
                for arg in args[1:]
                if isinstance(arg, ast.Constant)
            ]
            
        # 推断语言
        language = None
        if "LANGUAGE" in kwargs:
            lang_str = kwargs["LANGUAGE"]
            language = Language(lang_str.lower())
        else:
            if sources:
                first = str(sources[0])
                if first.endswith((".cpp", ".cxx", ".cc")):
                    language = Language.CPP
                elif first.endswith(".rs"):
                    language = Language.RUST
                elif first.endswith(".ts"):
                    language = Language.TYPESCRIPT
        if language is None:
            language = Language.CPP
            
        target = Target(
            name = target_name,
            type = TargetType.EXECUTABLE,
            language = language,
            sources = [SourceFile(Path(s), language) for s in sources if isinstance(s, str)],
            compile_options = {
                k: v for k, v in kwargs.items() if k not in ["SOURCES", "LANGUAGE"]
            },
            link_options = kwargs.get("LINK_OPTIONS", []),
            output_name = kwargs.get("OUTPUT_NAME", target_name),
        )
        self.project.targets.append(target)
        
    def _parse_keyword_arguments(self, node: ast.Call) -> Dict[str, Any]:
        kwargs = {}
        for kw in node.keywords:
            key = kw.arg
            value = self._get_constant_value(kw.value)
            kwargs[key] = value
        return kwargs
        
    def _get_constant_value(self, node):
        if isinstance(node, ast.Constant):
            return node.value
        if isinstance(node, ast.List):
            return [self._get_constant_value(elt) for elt in node.elts]
        if isinstance(node, ast.Str):
            return node.s
        if isinstance(node, ast.Name):
            return node.id
        return None