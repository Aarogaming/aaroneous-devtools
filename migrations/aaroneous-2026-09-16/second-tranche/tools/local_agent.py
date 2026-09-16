import sys
import json
import urllib.request
import os

if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass

# Auto-detect Ollama (11434) or LM Studio (1234)
def get_default_endpoint():
    env_url = os.environ.get("LOCAL_MODEL_URL")
    if env_url:
        return env_url
    # Check Ollama port 11434 first, then LM Studio 1234
    import socket
    for port in [11434, 1234]:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.settimeout(0.5)
            if s.connect_ex(("127.0.0.1", port)) == 0:
                return f"http://localhost:{port}/v1/chat/completions"
    return "http://localhost:11434/v1/chat/completions"

ENDPOINT = get_default_endpoint()
MODEL = os.environ.get("LOCAL_MODEL_NAME", "qwen3.5:9b-q4")

def query_local(prompt, file_path=None):
    content = prompt
    if file_path and os.path.exists(file_path):
        with open(file_path, "r", encoding="utf-8") as f:
            content += f"\n\n--- FILE CONTENTS ({file_path}) ---\n" + f.read()

    payload = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": "You are a specialized Rust systems engineer. Provide concise, production-ready code diffs adhering strictly to zero-ambient authority and zero-allocation hot paths."},
            {"role": "user", "content": content}
        ],
        "temperature": 0.2
    }
    
    req = urllib.request.Request(
        ENDPOINT,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            print(data["choices"][0]["message"]["content"])
    except Exception as e:
        print(f"ERROR: Local model endpoint unreachable at {ENDPOINT} (model: {MODEL}). Details: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python local_agent.py \"<prompt>\" [optional_file_path]")
        sys.exit(1)
    
    prompt = sys.argv[1]
    file_path = sys.argv[2] if len(sys.argv) > 2 else None
    query_local(prompt, file_path)
