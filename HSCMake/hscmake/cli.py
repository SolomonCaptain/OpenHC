import click
import pickle
import sys
from pathlib import Path

from .parser import HSCMakeParser
from .builder import BuildPlanner, BuildExecutor

@click.group()
def cli():
    """HSCMake - Hybrid Source Construction Make"""
    pass
    
@cli.command()
@click.option("--build-dir", default="build", help="Build directory")
@click.argument("config_file", default="HSCMakeList.txt", type=click.Path(exists=True))
def configure(build_dir, config_file):
    """Parse HSCMakeList.txt and generate build rules"""
    build_path = Path(build_dir)
    build_path.mkdir(exist_ok=True)
    
    parse = HSCMakeParser()
    project = parse.parse_file(Path(config_file))
    
    with open(build_path / ".hscmake_project.pkl", "wb") as f:
        pickle.dump(project, f)
        
    planner = BuildPlanner(project, build_path)
    planner.create_plan()
    click.echo(f"✅ Configured project '{project.name}' in {build_dir}")
    
@cli.command()
@click.option("--build-dir", default="build", help="Build directory")
@click.argument("targets", nargs=-1)
def build(build_dir, targets):
    """Build specified targets (or all if none)"""
    build_path = Path(build_dir)
    proj_file = build_path / ".hscmake_project.pkl"
    if not proj_file.exists():
        click.echo("❌ Project not configured. Run 'configure' first.", err=True)
        sys.exit(1)
        
    with open(proj_file, "rb") as f:
        project = pickle.load(f)
        
    planner = BuildPlanner(project, build_path)
    if targets:
        rules = planner.create_plan(target_names=list(targets))
    else:
        rules = planner.create_plan()
        
    executor = BuildExecutor()
    executor.execute(rules)
    click.echo("✅ Build completed.")
    
@cli.command()
@click.option("--build-dir", default="build")
def clean(build_dir):
    """Remove build directory"""
    build_path = Path(build_dir)
    if build_path.exists():
        import shutil
        shutil.rmtree(build_path)
        click.echo(f"🧹 Cleaned {build_dir}")
    else:
        click.echo(f"{build_dir} does not exist.")
        
@cli.command()
@click.option("--build-dir", default="build")
def list(build_dir):
    """List all available targets"""
    build_path = Path(build_dir)
    proj_file = build_path / ".hscmake_project.pkl"
    if not proj_file.exists():
        click.echo("❌ Project not configured. Run 'configure' first.", err=True)
        sys.exit(1)
        
    with open(proj_file, "rb") as f:
        project = pickle.load(f)
        
    for target in project.targets:
        click.echo(f"{target.name} ({target.language.value})")
        
def main():
    cli()
    
if __name__ == "__main__":
    main()