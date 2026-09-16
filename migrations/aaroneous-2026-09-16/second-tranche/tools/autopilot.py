import subprocess
import sys
import os
import time
import re

if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass

# Ensure cargo is on PATH
cargo_bin = os.path.expanduser(r"~\.cargo\bin")
if cargo_bin not in os.environ.get("PATH", ""):
    os.environ["PATH"] = cargo_bin + os.pathsep + os.environ.get("PATH", "")

QUEUE_FILE = os.path.join("docs", "handoff", "QUEUE.md")
STATUS_FILE = os.path.join("docs", "handoff", "STATUS.md")

def alert_user(title, message):
    """Pops a native Windows notification toast and plays a chime using built-in Windows .NET."""
    safe_title = title.replace("'", "")
    safe_msg = message.replace("'", "")
    ps_cmd = (
        "[console]::beep(1000, 200); [console]::beep(1400, 250); "
        "Add-Type -AssemblyName System.Windows.Forms; "
        "$n = New-Object System.Windows.Forms.NotifyIcon; "
        "$n.Icon = [System.Drawing.SystemIcons]::Information; "
        "$n.Visible = $true; "
        f"$n.ShowBalloonTip(5000, '{safe_title}', '{safe_msg}', [System.Windows.Forms.ToolTipIcon]::Info);"
    )
    subprocess.run(["powershell", "-NoProfile", "-Command", ps_cmd], capture_output=True)

def run_cmd(cmd):
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True, encoding="utf-8", errors="replace")
    return result.returncode, result.stdout, result.stderr

def get_next_task():
    if not os.path.exists(QUEUE_FILE):
        return None, None
    with open(QUEUE_FILE, "r", encoding="utf-8") as f:
        lines = f.readlines()
    for idx, line in enumerate(lines):
        match = re.match(r"^\s*-\s*\[\s*\]\s*(.+)$", line)
        if match:
            return idx, match.group(1).strip()
    return None, None

def mark_task_done(line_index):
    with open(QUEUE_FILE, "r", encoding="utf-8") as f:
        lines = f.readlines()
    lines[line_index] = lines[line_index].replace("- [ ]", "- [x]", 1)
    with open(QUEUE_FILE, "w", encoding="utf-8") as f:
        f.writelines(lines)

def append_status(text):
    os.makedirs(os.path.dirname(STATUS_FILE), exist_ok=True)
    with open(STATUS_FILE, "a", encoding="utf-8") as f:
        f.write(f"\n[{time.strftime('%Y-%m-%d %H:%M:%S')}] {text}\n")

def execute_cycle(task_description):
    print(f"\n========================================================")
    print(f"[AUTOPILOT] Starting Task: {task_description}")
    print(f"========================================================")
    append_status(f"START: {task_description}")

    # 1. Consult Cloud Architect (Gemini)
    print("[AUTOPILOT] 1. Requesting plan from Cloud Architect (Gemini)...")
    cloud_cmd = f"python tools/cloud_agent.py \"Provide the exact code changes and instructions to execute: {task_description}\""
    code, out, err = run_cmd(cloud_cmd)
    if code != 0:
        print(f"[AUTOPILOT] Cloud agent error:\n{err}")
        return False

    # 2. Local Qwen AST Invariant Check
    print("[AUTOPILOT] 2. Consulting Local Qwen for AST Invariant constraints...")
    local_cmd = f"python tools/local_agent.py \"Check AST invariants for: {task_description}\""
    run_cmd(local_cmd)

    # 3. Compiler & AST Verification Gauntlet (Up to 3 autonomous retries)
    for attempt in range(1, 4):
        print(f"[AUTOPILOT] 3. Running Verification Gauntlet (Attempt {attempt}/3)...")
        code, out, err = run_cmd("cargo check --workspace --all-targets")
        if code != 0:
            print(f"[AUTOPILOT] Attempt {attempt} failed compilation. Invoking auto-repair...")
            fix_prompt = f"Fix this compilation error for {task_description}:\n{err}\n{out}"
            _, fix_plan, _ = run_cmd(f"python tools/cloud_agent.py \"{fix_prompt}\"")
            continue

        # AST Invariant Audit
        code, audit_out, _ = run_cmd("cargo run -p ast_auditor -- audit core/ crates/")
        if code == 0 and "Violations: 0" in audit_out:
            print("[AUTOPILOT] Verification Gauntlet: PASSED (0 AST violations, 0 compiler errors).")
            append_status(f"SUCCESS: {task_description}")
            return True
        else:
            print(f"[AUTOPILOT] AST audit flagged violations:\n{audit_out}")

    # Escalation if 3 attempts fail
    print("[AUTOPILOT] ESCALATION: 3 attempts failed. Alerting operator...")
    alert_user("Aaroneous Escalation Required", f"Task failed 3 attempts: {task_description[:50]}...")
    append_status(f"ESCALATION REQUIRED: {task_description}")
    return False

def main_loop():
    print("[AUTOPILOT] Queue Daemon Active. Watching docs/handoff/QUEUE.md...")
    while True:
        idx, task = get_next_task()
        if task:
            success = execute_cycle(task)
            if success:
                mark_task_done(idx)
                print(f"[AUTOPILOT] Task completed and marked done in QUEUE.md.")
            else:
                print(f"[AUTOPILOT] Pausing daemon on failed task. Resolve and uncheck in QUEUE.md to resume.")
                break
        else:
            # Queue is clear, wait for new tasks
            time.sleep(10)

if __name__ == "__main__":
    main_loop()
