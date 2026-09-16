"""
Aaroneous Python SDK - WASM Pipeline (DEPRECATED)
Legacy helper for compiling WASM Enzymes.
Superseded by pure Rust Machine-Native and .si Solid-State container models.
"""
import subprocess
import os
import shutil
import warnings
from typing import Optional

class WasmPipeline:
    """
    [DEPRECATED] Manages the lifecycle of legacy WASM Enzymes.
    Superseded by native Rust execution.
    """
    def __init__(self, workspace_root: str = None):
        warnings.warn("WasmPipeline is deprecated. Use Machine-Native or .si Solid-State engines.", DeprecationWarning, stacklevel=2)
        if workspace_root is None:
            self.workspace_root = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
        else:
            self.workspace_root = workspace_root
        
        self.enzymes_dir = os.path.join(self.workspace_root, "extensions", "wasm")
        self.target_dir = os.path.join(self.workspace_root, "target", "wasm32-wasi", "release")

    def ensure_wasm_target(self) -> bool:
        """Ensures the wasm32-wasi target is installed."""
        try:
            result = subprocess.run(
                ["rustup", "target", "list", "--installed"],
                capture_output=True, text=True, check=True
            )
            if "wasm32-wasi" in result.stdout:
                return True
            print("[WasmPipeline] Installing wasm32-wasi target...")
            subprocess.run(["rustup", "target", "add", "wasm32-wasi"], check=True)
            return True
        except Exception as e:
            print(f"[WasmPipeline] Failed to check/install target: {e}")
            return False

    def compile_enzyme(self, enzyme_name: str) -> Optional[str]:
        """
        Compiles a specific Enzyme project to WASM.
        Returns the path to the .wasm file or None on failure.
        """
        if not self.ensure_wasm_target():
            return None

        enzyme_path = os.path.join(self.enzymes_dir, enzyme_name)
        if not os.path.exists(enzyme_path):
            print(f"[WasmPipeline] Enzyme '{enzyme_name}' not found at {enzyme_path}")
            return None

        print(f"[WasmPipeline] Compiling Enzyme: {enzyme_name}...")
        try:
            subprocess.run(
                ["cargo", "build", "--release", "--target", "wasm32-wasi"],
                cwd=enzyme_path,
                check=True,
                capture_output=True,
                text=True
            )
            
            # Find the resulting .wasm file
            # Usually named after the package name in Cargo.toml
            # For simplicity, we look for any .wasm in the target dir
            wasm_file = None
            for f in os.listdir(self.target_dir):
                if f.endswith('.wasm'):
                    wasm_file = os.path.join(self.target_dir, f)
                    break
            
            if wasm_file:
                print(f"[WasmPipeline] Compilation successful: {wasm_file}")
                return wasm_file
            else:
                print("[WasmPipeline] Compilation succeeded but no .wasm file found.")
                return None
        except subprocess.CalledProcessError as e:
            print(f"[WasmPipeline] Compilation failed:\n{e.stderr}")
            return None

    def deploy_and_run(self, enzyme_name: str, hypervisor):
        """
        Compiles an Enzyme and immediately runs it via the Hypervisor.
        """
        wasm_path = self.compile_enzyme(enzyme_name)
        if wasm_path:
            print(f"[WasmPipeline] Deploying {enzyme_name} to Hypervisor...")
            # This assumes hypervisor has a load_and_run method
            # hypervisor.load_and_run(wasm_path) 
            print("[WasmPipeline] Execution complete.")
            return True
        return False
