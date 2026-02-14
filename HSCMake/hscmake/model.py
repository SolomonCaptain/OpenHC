from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import List, Dict, Any, Optional

class TargetType(Enum):
    EXECUTABLE = "executable"
    LIBRARY = "library"
    TEST = "test"
    
class Language(Enum):
    CPP = "cpp"
    RUST = "rust"
    TYPESCRIPT = "typescript"
    
@dataclass
class SourceFile:
    path: Path
    language: Language
    
@dataclass
class Target:
    name: str
    type: TargetType
    language: Language
    sources: List[SourceFile] = field(default_factory=list)
    compile_options: Dict[str, Any] = field(default_factory=dict)
    link_options: List[str] = field(default_factory=list)
    dependencies: List[str] = field(default_factory=list)
    output_name: Optional[str] = None
    
@dataclass
class Project:
    name: str
    version: str = "0.1.0"
    languages: List[Language] = field(default_factory=list)
    targets: List[Target] = field(default_factory=list)
    build_dir: Path = Path("build")
    
    def get_target(self, name: str) -> Optional[Target]:
        for t in self.targets:
            if t.name == name:
                return t
        return None