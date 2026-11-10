# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_proxmox_mcp_rs_global_optspecs
	string join \n c/config= H/host= p/port= u/user= P/password= n/token-name= v/token-value= k/no-verify-ssl L/log-level= log-file-enable log-dir= log-filename= log-rotate= t/server-type= http-host= l/http-port= http-auth-token= lazy-mode h/help V/version
end

function __fish_proxmox_mcp_rs_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_proxmox_mcp_rs_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_proxmox_mcp_rs_using_subcommand
	set -l cmd (__fish_proxmox_mcp_rs_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s c -l config -d 'Config file path' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s H -l host -d 'Proxmox Host (e.g., 192.168.1.10)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s p -l port -d 'Proxmox Port (default: 8006)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s u -l user -d 'Proxmox User (e.g., root@pam)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s P -l password -d 'Proxmox Password' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s n -l token-name -d 'API Token Name (e.g., mytoken)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s v -l token-value -d 'API Token Value (UUID)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s L -l log-level -d 'Log level (error, warn, info, debug, trace)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -l log-dir -d 'Log file directory' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -l log-filename -d 'Log filename prefix' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -l log-rotate -d 'Log rotation strategy (daily, hourly, never)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s t -l server-type -d 'Server type (stdio or http)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -l http-host -d 'HTTP Host (only for http type)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s l -l http-port -d 'HTTP Port (only for http type)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -l http-auth-token -d 'HTTP Auth Token (only for http type)' -r
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s k -l no-verify-ssl -d 'Disable SSL verification (for self-signed certs)'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -l log-file-enable -d 'Enable logging to a file'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -l lazy-mode -d 'Enable Lazy Loading mode (starts with minimal tools)'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s h -l help -d 'Print help'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -s V -l version -d 'Print version'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -f -a "completions" -d 'Generate shell completion scripts'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_using_subcommand completions" -s h -l help -d 'Print help'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_using_subcommand help; and not __fish_seen_subcommand_from completions help" -f -a "completions" -d 'Generate shell completion scripts'
complete -c proxmox-mcp-rs -n "__fish_proxmox_mcp_rs_using_subcommand help; and not __fish_seen_subcommand_from completions help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
