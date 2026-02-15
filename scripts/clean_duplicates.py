import sys

def clean_duplicates():
    path = 'src/mcp.rs'
    with open(path, 'r') as f:
        lines = f.readlines()

    new_lines = []
    in_call_tool = False
    match_count = 0
    
    for line in lines:
        if 'pub async fn call_tool' in line:
            in_call_tool = True
            match_count += 1
        
        if in_call_tool and match_count > 1:
            if line.strip() == '}':
                in_call_tool = False
            continue
            
        new_lines.append(line)

    with open(path, 'w') as f:
        f.writelines(new_lines)

if __name__ == "__main__":
    clean_duplicates()
