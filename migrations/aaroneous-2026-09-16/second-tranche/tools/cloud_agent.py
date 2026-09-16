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

def load_env_file():
    if "GEMINI_API_KEY" in os.environ:
        return
    candidates = [
        os.path.join(os.path.expanduser("~"), ".env"),
        os.path.join(os.path.dirname(__file__), "..", ".env"),
        ".env"
    ]
    for env_path in candidates:
        if os.path.exists(env_path):
            try:
                with open(env_path, "r", encoding="utf-8") as f:
                    for line in f:
                        line = line.strip()
                        if line and not line.startswith("#") and "=" in line:
                            k, v = line.split("=", 1)
                            k = k.strip()
                            v = v.strip().strip("'\"")
                            if k not in os.environ:
                                os.environ[k] = v
            except Exception:
                pass

load_env_file()
API_KEY = os.environ.get("GEMINI_API_KEY")
MODEL = os.environ.get("GEMINI_MODEL", "gemini-2.5-flash")
ENDPOINT = f"https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={API_KEY}"

def query_gemini(prompt, file_path=None):
    if not API_KEY:
        print("ERROR: GEMINI_API_KEY environment variable is not set.", file=sys.stderr)
        print("Set it in PowerShell via: $env:GEMINI_API_KEY = 'your_api_key_here'", file=sys.stderr)
        sys.exit(1)

    content = prompt
    if file_path and os.path.exists(file_path):
        with open(file_path, "r", encoding="utf-8") as f:
            content += f"\n\n--- TARGET FILE ({file_path}) ---\n" + f.read()

    payload = {
        "contents": [
            {
                "role": "user",
                "parts": [{"text": content}]
            }
        ],
        "systemInstruction": {
            "parts": [{
                "text": "You are the Lead Systems Architect for the Aaroneous sovereign Rust microkernel. Provide concise, production-ready Rust code diffs, module declarations, and concrete shell actions adhering strictly to zero-ambient authority."
            }]
        },
        "generationConfig": {
            "temperature": 0.2
        }
    }

    req = urllib.request.Request(
        ENDPOINT,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )

    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            text = data["candidates"][0]["content"]["parts"][0]["text"]
            print(text)
    except Exception as e:
        print(f"ERROR querying Gemini API: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python cloud_agent.py \"<prompt>\" [optional_file_path]")
        sys.exit(1)

    prompt = sys.argv[1]
    file_path = sys.argv[2] if len(sys.argv) > 2 else None
    query_gemini(prompt, file_path)
