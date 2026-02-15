import subprocess
import json
import sys
import os

def rpc(method, params=None, id=1):
    return {"jsonrpc": "2.0", "method": method, "params": params, "id": id}

def count_tokens(text):
    # Rough estimate: 1 token per 4 characters
    return len(text) / 4

def benchmark():
    binary = "./target/release/proxmox-mcp-rs"
    if not os.path.exists(binary):
        print(f"Error: Binary {binary} not found. Please run 'cargo build --release' first.")
        sys.exit(1)

    # Set up dummy environment for the server to start even without real Proxmox
    env = os.environ.copy()
    env["PROXMOX_HOST"] = "localhost"
    env["PROXMOX_USER"] = "root@pam"
    # Use tokens to avoid startup login attempt
    env["PROXMOX_TOKEN_NAME"] = "benchmark"
    env["PROXMOX_TOKEN_VALUE"] = "00000000-0000-0000-0000-000000000000"
    env.pop("PROXMOX_PASSWORD", None)

    process = subprocess.Popen(
        [binary],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr,
        text=True,
        bufsize=0,
        env=env
    )

    def send_and_receive(req):
        try:
            process.stdin.write(json.dumps(req) + "\n")
            line = process.stdout.readline()
            if not line:
                return None
            return json.loads(line)
        except Exception as e:
            print(f"Error sending/receiving: {e}")
            return None

    # 1. Initialize
    send_and_receive(rpc("initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "benchmark", "version": "1.0"}
    }))

    # 2. List Tools
    list_tools_resp = send_and_receive(rpc("tools/list"))
    if not list_tools_resp:
        print("Failed to get tools/list")
        sys.exit(1)
    list_tools_text = json.dumps(list_tools_resp)
    
    # 3. List Resources
    list_resources_resp = send_and_receive(rpc("resources/list"))
    if not list_resources_resp:
        print("Failed to get resources/list")
        sys.exit(1)
    list_resources_text = json.dumps(list_resources_resp)

    process.terminate()

    print(f"\n--- Baseline Benchmarks ---")
    print(f"tools/list response size: {len(list_tools_text)} characters (~{count_tokens(list_tools_text):.1f} tokens)")
    print(f"resources/list response size: {len(list_resources_text)} characters (~{count_tokens(list_resources_text):.1f} tokens)")

if __name__ == "__main__":
    benchmark()
