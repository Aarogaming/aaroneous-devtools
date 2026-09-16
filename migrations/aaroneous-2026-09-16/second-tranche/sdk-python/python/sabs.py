"""
Aaroneous Python SDK - SAB Manager
Provides Python Orchestrators with direct access to the SAB (Specialized Agent Body) framework.
Maps directly to the Rust SAB Matrix via the Shared Memory Synapse.
"""
import json
import os
from .synapse_bridge import SynapseBridge

class SabManager:
    """
    Manages SAB manifests and matrices.
    Allows Python to define, load, and activate SABs in the Rust Core.
    """
    def __init__(self, bridge: SynapseBridge):
        self.bridge = bridge
        self.sab_matrix_path = os.path.join(os.path.dirname(__file__), '..', '..', 'registry', 'sab_matrix.json')

    def load_matrix(self) -> dict:
        """Loads the current SAB Matrix from the registry."""
        if not os.path.exists(self.sab_matrix_path):
            raise FileNotFoundError(f"SAB Matrix not found at {self.sab_matrix_path}")
        
        with open(self.sab_matrix_path, 'r') as f:
            return json.load(f)

    def activate_sab(self, sab_id: str):
        """
        Sends a command to the Synapse to activate a specific SAB.
        The Rust Core will pick this up and load the corresponding module.
        """
        # Write activation command to a known offset in the Synapse
        # Format: [SAB_ACTIVATE][SAB_ID_LENGTH][SAB_ID]
        payload = f"ACTIVATE:{sab_id}".encode('utf-8')
        self.bridge.write_command(200, payload)
        print(f"[SabManager] Activated SAB: {sab_id}")

    def get_sab_status(self) -> str:
        """Reads the current active SAB status from the Synapse."""
        # In a real implementation, this would read a specific memory offset
        # For now, we simulate reading a status string
        return "ACTIVE"
