#!/usr/bin/env python3
"""
Aaroneous Python Orchestrator
Connects to the Rust daemon via Synapse Bridge and orchestrates tasks.
"""

import sys
import time
import json
import argparse
from pathlib import Path

# Add SDK to path
sys.path.insert(0, str(Path(__file__).parent.parent / "sdk" / "python"))

from synapse_bridge import (
    SynapseBridge,
    SynapseMessage,
    ComputeResult,
    MetabolicState,
    create_bridge,
    send_compute_task,
    submit_task,
)


class AaroneousOrchestrator:
    """High-level orchestrator that manages tasks and monitors Aaroneous health."""

    def __init__(self, synapse_name: str = "SAB_STORE"):
        self.bridge = create_bridge(synapse_name)
        self.task_counter = 0

    def submit_code_task(self, description: str, priority: float = 0.5) -> dict:
        """Submit a code-related task to the decision engine."""
        self.task_counter += 1
        task_id = f"py_task_{self.task_counter}_{int(time.time())}"

        self.bridge.write_task(
            task_id=task_id,
            task_type="code_generation",
            description=description,
            priority=priority,
        )

        print(f"[Orchestrator] Submitted task: {task_id}")
        print(f"  Description: {description}")
        print(f"  Priority: {priority}")

        return {"task_id": task_id, "status": "submitted"}

    def request_compute(self, task_type: str, input_data: list) -> ComputeResult:
        """Request a compute task (Monte Carlo, entropy, etc.)."""
        self.bridge.write_compute_request(task_type, input_data)

        # Wait for result
        start = time.time()
        while time.time() - start < 5.0:
            result = self.bridge.read_compute_result()
            if result:
                print(f"[Orchestrator] Compute result: {result.task_type}")
                print(f"  Results: {result.results[:5]}...")
                print(f"  Confidence: {result.confidence:.3f}")
                return result
            time.sleep(0.1)

        print("[Orchestrator] Compute request timed out")
        return None

    def check_metabolic_health(self) -> MetabolicState:
        """Check current metabolic state of Aaroneous."""
        state = self.bridge.read_metabolic_state()
        if state:
            print(f"[Orchestrator] Metabolic Health:")
            print(f"  Tokens: {state.global_tokens:.1f}")
            print(f"  Expression Rate: {state.expression_rate:.2f}")
            print(f"  Throttle: {state.throttle_state}")
            print(f"  Risk Score: {state.risk_score:.2f}")
        return state

    def monitor_loop(self, interval: float = 5.0):
        """Continuously monitor the synapse for updates."""
        print(f"[Orchestrator] Starting monitor loop (interval: {interval}s)")
        print("Press Ctrl+C to stop\n")

        try:
            while True:
                # Check for task evaluations
                evaluation = self.bridge.read_task_evaluation()
                if evaluation:
                    print(f"\n[Orchestrator] Task Evaluation Received:")
                    print(f"  Task ID: {evaluation.get('task_id', 'unknown')}")
                    print(f"  Confidence: {evaluation.get('confidence', 0):.3f}")
                    print(f"  Action: {evaluation.get('recommended_action', 'unknown')}")
                    print(f"  Reasoning: {evaluation.get('reasoning', '')}")

                # Check metabolic state
                state = self.check_metabolic_health()

                time.sleep(interval)

        except KeyboardInterrupt:
            print("\n[Orchestrator] Monitor stopped by user")

    def close(self):
        """Clean up resources."""
        self.bridge.disconnect()


def main():
    parser = argparse.ArgumentParser(description="Aaroneous Python Orchestrator")
    parser.add_argument(
        "--mode",
        choices=["submit", "compute", "monitor", "health"],
        default="health",
        help="Operation mode",
    )
    parser.add_argument("--description", type=str, help="Task description")
    parser.add_argument("--priority", type=float, default=0.5, help="Task priority (0.0-1.0)")
    parser.add_argument("--compute-type", type=str, help="Compute task type (monte_carlo, entropy, etc.)")
    parser.add_argument("--input-data", type=str, help="JSON array of input data")
    parser.add_argument("--interval", type=float, default=5.0, help="Monitor interval in seconds")

    args = parser.parse_args()

    orchestrator = AaroneousOrchestrator()

    try:
        if args.mode == "submit":
            if not args.description:
                print("Error: --description is required for submit mode")
                sys.exit(1)
            result = orchestrator.submit_code_task(args.description, args.priority)
            print(f"\nResult: {json.dumps(result, indent=2)}")

        elif args.mode == "compute":
            if not args.compute_type:
                print("Error: --compute-type is required for compute mode")
                sys.exit(1)

            input_data = json.loads(args.input_data) if args.input_data else [0.5, 0.3, 0.7]
            result = orchestrator.request_compute(args.compute_type, input_data)
            if result:
                print(f"\nResult: {json.dumps(result.__dict__, indent=2, default=str)}")

        elif args.mode == "monitor":
            orchestrator.monitor_loop(args.interval)

        elif args.mode == "health":
            state = orchestrator.check_metabolic_health()
            if state:
                print(f"\nMetabolic State: {json.dumps(state.__dict__, indent=2)}")
            else:
                print("No metabolic state available")

    finally:
        orchestrator.close()


if __name__ == "__main__":
    main()
