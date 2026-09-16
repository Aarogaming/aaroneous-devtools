"""
Aaroneous Python SDK - Synapse Bridge
Provides the byte-level interface for Python Orchestrators to communicate with the Rust Core.
"""
from .synapse_bridge import (
    SynapseBridge,
    SynapseMessage,
    ComputeResult,
    MetabolicState,
    create_bridge,
    send_compute_task,
    submit_task,
)

__all__ = [
    "SynapseBridge",
    "SynapseMessage",
    "ComputeResult",
    "MetabolicState",
    "create_bridge",
    "send_compute_task",
    "submit_task",
]
