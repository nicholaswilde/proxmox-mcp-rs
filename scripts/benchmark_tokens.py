import subprocess
import json
import sys
import os

def rpc(method, params=None, id=1):
    return {"jsonrpc": "2.0", "method": method, "params": params, "id": id}

def count_tokens(text):
    return len(text) / 4

def benchmark():
    binary = "./target/release/proxmox-mcp-rs"
    env = os.environ.copy()
    env["PROXMOX_HOST"] = "localhost"
    env["PROXMOX_USER"] = "root@pam"
    env["PROXMOX_TOKEN_NAME"] = "benchmark"
    env["PROXMOX_TOKEN_VALUE"] = "00000000-0000-0000-0000-000000000000"
    env["PROXMOX_NO_VERIFY_SSL"] = "true"
    env.pop("PROXMOX_PASSWORD", None)

    process = subprocess.Popen(
        [binary],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=0,
        env=env
    )

    def receive():
        line = process.stdout.readline()
        if not line:
            return None
        try:
            data = json.loads(line)
            if "method" in data and data["method"].startswith("notifications/"):
                return receive()
            return data
        except:
            return None

    def send(req):
        process.stdin.write(json.dumps(req) + "\n")

    # 1. Initialize
    send(rpc("initialize", {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "benchmark", "version": "1.0"}
    }))
    receive()

    # 2. Trigger full tool load (if lazy)
    send(rpc("tools/call", {"name": "load_all_tools", "arguments": {}}))
    receive()

    # 3. Benchmark tools/list
    send(rpc("tools/list"))
    resp = receive()
    
    if resp and "result" in resp:
        tools_text = json.dumps(resp["result"])
        char_count = len(tools_text)
        token_count = char_count / 4
        print(f"Tools List Size: {char_count} chars (~{token_count:.0f} tokens)")
        print(f"Tool Count: {len(resp['result'].get('tools', []))}")
    else:
        print("Failed to get tools list")
    
    process.terminate()

if __name__ == "__main__":
    benchmark()
