# Implementation Plan: PCI/USB Passthrough Tools

## Phase 1: Hardware Listing
- [x] Task: Create `src/proxmox/hardware.rs`.
- [x] Task: Implement `get_pci_devices` and `get_usb_devices` in `src/proxmox/hardware.rs`.
- [x] Task: Add `list_pci_devices` and `list_usb_devices` tools to `src/mcp.rs`.
- [x] Task: Update `src/proxmox/mod.rs` to include the new module.

## Phase 2: Device Management (VM)
- [x] Task: Update `src/proxmox/vm.rs` to implement `add_pci_device` and `add_usb_device`.
    - `add_pci_device` should handle `hostpciX`.
    - `add_usb_device` should handle `usbX` (host or spice).
- [x] Task: Implement `remove_vm_device` in `src/proxmox/vm.rs` (generic removal for `hostpciX`, `usbX`).
- [x] Task: Add `add_pci_device`, `add_usb_device`, and `remove_vm_device` tools to `src/mcp.rs`.

## Phase 3: Testing & Verification
- [x] Task: Add unit tests in `src/tests.rs` mocking the hardware list endpoints and config update endpoints.
- [ ] Task: Manual verification using `config.toml` (non-destructive listing, cautious adding).