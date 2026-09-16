import mmap
import os
import struct
import time
import json
from typing import Optional, Dict, Any, List
from dataclasses import dataclass, asdict

# Synapse Protocol Constants
MAGIC_BYTES = b'\xAA\x55\xAA\x55'
HEADER_SIZE = 16  # Magic(4) + Version(4) + Status(4) + Checksum(4)
STATUS_OFFSET = 8
VERSION_OFFSET = 4
CHECKSUM_OFFSET = 12

@dataclass
class SynapseMessage:
    """Structured message for synapse communication"""
    msg_type: str
    payload: Dict[str, Any]
    timestamp: float = 0.0
    sequence: int = 0
    
    def __post_init__(self):
        if self.timestamp == 0.0:
            self.timestamp = time.time()
    
    def to_bytes(self) -> bytes:
        """Serialize message to bytes for synapse writing"""
        data = json.dumps(asdict(self)).encode('utf-8')
        return data
    
    @classmethod
    def from_bytes(cls, data: bytes) -> 'SynapseMessage':
        """Deserialize message from synapse bytes"""
        decoded = json.loads(data.decode('utf-8'))
        return cls(**decoded)

@dataclass
class ComputeResult:
    """Result from compute engine execution"""
    task_type: str
    results: List[float]
    confidence: float
    execution_time_ms: float

@dataclass
class MetabolicState:
    """Current metabolic state of the system"""
    global_tokens: float
    expression_rate: float
    throttle_state: str
    specialist_count: int
    risk_score: float = 0.0

class SynapseBridge:
    """
    Python-side bridge to the Aaroneous Shared Memory Synapse.
    Allows Python Orchestrators to read/write byte-level data directly to the Rust Core.
    Supports structured messaging, compute requests, and metabolic monitoring.
    """
    
    def __init__(self, name: str = "SAB_STORE", size: int = 1024 * 1024):
        self.name = name
        self.path = os.path.join(os.environ.get("LOCALAPPDATA", ""), "Temp", f"{name}.synapse")
        self.size = size
        self.mm = None
        self.f = None
        self.sequence = 0
        self.connected = False
    
    def connect(self) -> 'SynapseBridge':
        """Connect to the shared memory synapse"""
        if not os.path.exists(self.path):
            # Create the synapse file if it doesn't exist
            with open(self.path, "wb") as f:
                # Write header
                f.write(MAGIC_BYTES)
                f.write(struct.pack('<I', 1))  # Version
                f.write(struct.pack('<I', 0))  # Status
                f.write(struct.pack('<I', 0))  # Checksum
                # Fill rest with zeros
                f.write(b"\x00" * (self.size - HEADER_SIZE))
        
        self.f = open(self.path, "r+b")
        self.mm = mmap.mmap(self.f.fileno(), self.size)
        self.connected = True
        
        # Verify magic bytes
        self.mm.seek(0)
        magic = self.mm.read(4)
        if magic != MAGIC_BYTES:
            raise RuntimeError(f"Invalid synapse magic bytes: {magic}")
        
        print(f"[SynapseBridge] Linked to {self.path}")
        return self
    
    def disconnect(self):
        """Disconnect from the synapse"""
        if self.mm:
            self.mm.close()
        if self.f:
            self.f.close()
        self.connected = False
        print("[SynapseBridge] Disconnected")
    
    def _calculate_checksum(self, data: bytes) -> int:
        """Simple checksum for data integrity"""
        return sum(data) & 0xFFFFFFFF
    
    def _write_header(self, status: int, version: int = 1):
        """Write synapse header"""
        self.mm.seek(0)
        self.mm.write(MAGIC_BYTES)
        self.mm.write(struct.pack('<I', version))
        self.mm.write(struct.pack('<I', status))
    
    def _read_header(self) -> Dict[str, int]:
        """Read synapse header"""
        self.mm.seek(0)
        magic = self.mm.read(4)
        version = struct.unpack('<I', self.mm.read(4))[0]
        status = struct.unpack('<I', self.mm.read(4))[0]
        checksum = struct.unpack('<I', self.mm.read(4))[0]
        return {
            'magic': magic,
            'version': version,
            'status': status,
            'checksum': checksum
        }
    
    def write_message(self, message: SynapseMessage, offset: int = HEADER_SIZE) -> bool:
        """Write a structured message to the synapse"""
        if not self.connected:
            raise RuntimeError("Not connected to synapse")
        
        data = message.to_bytes()
        message.sequence = self.sequence
        self.sequence += 1
        
        # Write data
        self.mm.seek(offset)
        self.mm.write(data)
        
        # Update header with status
        self._write_header(status=1)  # 1 = data available
        
        return True
    
    def read_message(self, offset: int = HEADER_SIZE, max_size: int = 4096) -> Optional[SynapseMessage]:
        """Read a structured message from the synapse"""
        if not self.connected:
            raise RuntimeError("Not connected to synapse")
        
        header = self._read_header()
        if header['status'] == 0:
            return None  # No data available
        
        self.mm.seek(offset)
        data = self.mm.read(max_size)
        
        # Find null terminator
        null_idx = data.find(b'\x00')
        if null_idx > 0:
            data = data[:null_idx]
        
        if not data:
            return None
        
        try:
            return SynapseMessage.from_bytes(data)
        except Exception as e:
            print(f"[SynapseBridge] Failed to parse message: {e}")
            return None
    
    def write_compute_request(self, task_type: str, input_data: List[float]) -> bool:
        """Send a compute request to the Rust core"""
        message = SynapseMessage(
            msg_type=f"compute:{task_type}",
            payload={"input": input_data}
        )
        return self.write_message(message)
    
    def read_compute_result(self) -> Optional[ComputeResult]:
        """Read a compute result from the synapse"""
        message = self.read_message()
        if message and message.msg_type.startswith("result:"):
            return ComputeResult(
                task_type=message.msg_type.split(":")[1],
                results=message.payload.get("results", []),
                confidence=message.payload.get("confidence", 0.5),
                execution_time_ms=message.payload.get("execution_time_ms", 0.0)
            )
        return None
    
    def read_metabolic_state(self) -> Optional[MetabolicState]:
        """Read current metabolic state from synapse"""
        message = self.read_message()
        if message and message.msg_type == "metabolic:state":
            return MetabolicState(
                global_tokens=message.payload.get("global_tokens", 0.0),
                expression_rate=message.payload.get("expression_rate", 1.0),
                throttle_state=message.payload.get("throttle_state", "normal"),
                specialist_count=message.payload.get("specialist_count", 0),
                risk_score=message.payload.get("risk_score", 0.0)
            )
        return None
    
    def write_task(self, task_id: str, task_type: str, description: str, priority: float = 0.5) -> bool:
        """Submit a task to the decision engine"""
        message = SynapseMessage(
            msg_type="task:submit",
            payload={
                "task_id": task_id,
                "task_type": task_type,
                "description": description,
                "priority": priority
            }
        )
        return self.write_message(message)
    
    def read_task_evaluation(self) -> Optional[Dict[str, Any]]:
        """Read a task evaluation from the decision engine"""
        message = self.read_message()
        if message and message.msg_type == "task:evaluation":
            return message.payload
        return None
    
    def write_command(self, offset: int, data: bytes):
        """Low-level write to synapse (legacy support)"""
        if not self.connected:
            raise RuntimeError("Not connected")
        self.mm.seek(offset)
        self.mm.write(data)
    
    def read_status(self) -> int:
        """Read synapse status (legacy support)"""
        if not self.connected:
            raise RuntimeError("Not connected")
        header = self._read_header()
        return header['status']
    
    def __enter__(self):
        return self.connect()
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.disconnect()
        return False


# Convenience functions for common operations
def create_bridge(name: str = "SAB_STORE", size: int = 1024 * 1024) -> SynapseBridge:
    """Create and connect a new synapse bridge"""
    return SynapseBridge(name, size).connect()


def send_compute_task(bridge: SynapseBridge, task_type: str, input_data: List[float]) -> bool:
    """Send a compute task and wait for result"""
    bridge.write_compute_request(task_type, input_data)
    
    # Wait for result (with timeout)
    start = time.time()
    while time.time() - start < 5.0:
        result = bridge.read_compute_result()
        if result:
            return result
        time.sleep(0.1)
    
    return None


def submit_task(bridge: SynapseBridge, task_id: str, task_type: str, description: str, priority: float = 0.5) -> bool:
    """Submit a task and wait for evaluation"""
    bridge.write_task(task_id, task_type, description, priority)
    
    # Wait for evaluation
    start = time.time()
    while time.time() - start < 10.0:
        evaluation = bridge.read_task_evaluation()
        if evaluation:
            return evaluation
        time.sleep(0.1)
    
    return None
