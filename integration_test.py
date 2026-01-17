import sys
import json
import subprocess
import os

# Configuration
BINARY_PATH = "./target/debug/proxmox-mcp-rs"
CONFIG_PATH = "./config.toml"

def create_request(method, params=None, req_id=1):
    return {
        "jsonrpc": "2.0",
        "method": method,
        "params": params or {},
        "id": req_id
    }

def run_test():
    if not os.path.exists(BINARY_PATH):
        print(f"Error: Binary not found at {BINARY_PATH}")
        sys.exit(1)

    print(f"Starting {BINARY_PATH} with config {CONFIG_PATH}...")
    
    # Start the process
    process = subprocess.Popen(
        [BINARY_PATH, "--config", CONFIG_PATH],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr, # Pipe stderr to console for debugging
        text=True,
        bufsize=0  # Unbuffered
    )

    try:
        # 1. Send Initialize
        init_req = create_request("initialize", {
            "protocolVersion": "0.1.0",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0"}
        }, 1)
        
        print("-> Sending initialize...")
        process.stdin.write(json.dumps(init_req) + "\n")
        process.stdin.flush()
        
        response = process.stdout.readline()
        if not response:
            raise Exception("No response from server")
            
        print("<- Received initialize response")
        resp_json = json.loads(response)
        if "error" in resp_json:
             print(f"Server returned error: {resp_json['error']}")
             return

        # 2. Send Initialized Notification
        print("-> Sending notifications/initialized...")
        init_notif = {
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }
        process.stdin.write(json.dumps(init_notif) + "\n")
        process.stdin.flush()

        # 3. List Tools
        print("-> Requesting tools/list...")
        list_req = create_request("tools/list", {}, 2)
        process.stdin.write(json.dumps(list_req) + "\n")
        process.stdin.flush()
        
        response = process.stdout.readline()
        tools_resp = json.loads(response)
        
        if "result" in tools_resp:
            tools = tools_resp["result"].get("tools", [])
            print(f"<- Success! Found {len(tools)} tools.")
            # Verify specific tool exists
            if any(t['name'] == 'list_nodes' for t in tools):
                 print("   Verified 'list_nodes' tool is present.")
            else:
                 print("   WARNING: 'list_nodes' tool missing!")
        else:
            print("<- Error listing tools:", tools_resp)

        # 4. Call a safe Tool (list_nodes)
        print("-> Calling tool 'list_nodes' (Connects to Proxmox)...")
        call_req = create_request("tools/call", {
            "name": "list_nodes",
            "arguments": {}
        }, 3)
        process.stdin.write(json.dumps(call_req) + "\n")
        process.stdin.flush()
        
        response = process.stdout.readline()
        call_resp = json.loads(response)
        
        if "result" in call_resp:
            content = call_resp["result"]["content"][0]["text"]
            print(f"<- Proxmox Response:\n{content}")
        elif "error" in call_resp:
            print(f"<- Proxmox API Error: {call_resp['error']}")

    except Exception as e:
        print(f"Test failed: {e}")
    finally:
        process.terminate()

if __name__ == "__main__":
    run_test()
