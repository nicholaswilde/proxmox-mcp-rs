# Specification: PCI/USB Passthrough Tools

## Goal
Enable users to configure PCI (e.g., GPU) and USB device passthrough for QEMU VMs via MCP tools.

## Features
1.  **List PCI Devices**: List available PCI devices on a node (using `pvesh get /nodes/{node}/hardware/pci`).
2.  **List USB Devices**: List available USB devices on a node (using `pvesh get /nodes/{node}/hardware/usb`).
3.  **Add PCI Device**: Add a PCI device to a VM (hostpci0, hostpci1...).
4.  **Add USB Device**: Add a USB device to a VM (usb0, usb1...).
5.  **Remove PCI/USB Device**: Remove a device from a VM configuration.

## API Endpoints
-   GET `/nodes/{node}/hardware/pci`: List PCI devices.
-   GET `/nodes/{node}/hardware/usb`: List USB devices.
-   PUT `/nodes/{node}/qemu/{vmid}/config`: Update VM config (add/remove devices).
    -   PCI parameter: `hostpci[n]=...`
    -   USB parameter: `usb[n]=...`

## Tools to Add
-   `list_pci_devices`: List available PCI devices on a node.
-   `list_usb_devices`: List available USB devices on a node.
-   `add_pci_device`: Add a PCI device to a VM.
-   `add_usb_device`: Add a USB device to a VM.
-   `remove_vm_device`: Remove a PCI or USB device from a VM.
