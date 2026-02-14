import pickle
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
from typing import List, Optional

from .model import Language, Project, Target
from .rules import BuildRule, CppNinjaRule, RustCargoRule, TypeScriptTSCRule


class BuildPlanner:
    def __init__(self, project: Project, build_dir: Path):
        self.project = project
        self.build_dir = build_dir
        self.rules: List[BuildRule] = []

    def create_plan(self, target_names: Optional[List[str]] = None) -> List[BuildRule]:
        if target_names is None:
            targets = self.project.targets
        else:
            targets = [self.project.get_target(name) for name in target_names]
        self.rules = []
        for target in targets:
            if target is None:
                continue
            rule = self._create_rule(target)
            rule.generate()
            self.rules.append(rule)
        return self.rules

    def _create_rule(self, target: Target) -> BuildRule:
        if target.language == Language.CPP:
            return CppNinjaRule(target, self.build_dir)
        elif target.language == Language.RUST:
            return RustCargoRule(target, self.build_dir)
        elif target.language == Language.TYPESCRIPT:
            return TypeScriptTSCRule(target, self.build_dir)
        else:
            raise ValueError(f"Unsupported language: {target.language}")


class BuildExecutor:
    def __init__(self, max_workers: int = None):
        self.max_workers = max_workers or 4

    def execute(self, rules: List[BuildRule]):
        with ThreadPoolExecutor(max_workers=self.max_workers) as executor:
            future_to_rule = {
                executor.submit(rule.build): rule for rule in rules
            }
            for future in as_completed(future_to_rule):
                rule = future_to_rule[future]
                try:
                    future.result()
                    print(f"✅ Built {rule.target.name}")
                except Exception as e:
                    print(f"❌ Failed to build {rule.target.name}: {e}")