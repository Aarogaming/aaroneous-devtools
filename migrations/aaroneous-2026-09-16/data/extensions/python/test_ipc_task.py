"""
Synaptic Task Proof: Python Orchestrator -> Rust Executor
Writes a command to the Synapse and waits for a response.
"""
import sys
import os
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..', 'sdk', 'python')))

from synapse_bridge import SynapseBridge
import time

def run_synaptic_task():
    bridge = SynapseBridge().connect()
    print("[Python] Sending command to Synapse...")
    
    # Write a command (e.g., "CALCULATE") at offset 100
    bridge.write_command(100, b"CALCULATE\x00")
    
    # Wait for Rust to process and update status
    print("[Python] Waiting for Rust Executor to respond...")
    for _ in range(10):
        status = bridge.read_status()
        if status == 1: # 1 = Completed
            print("[Python] Task completed! Rust Executor responded.")
            break
        time.sleep(0.5)
    else:
        print("[Python] Timeout waiting for response.")
    
    bridge.close()

if __name__ == "__main__":
    run_synaptic_task()
