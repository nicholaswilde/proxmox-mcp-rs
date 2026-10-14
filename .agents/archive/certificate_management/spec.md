# Certificate Management Specification

## Goal
Manage SSL certificates for Proxmox nodes, including custom uploads and ACME (Let's Encrypt) integration.

## New Tools

### `list_certificates`
- **Description:** List certificates installed on a node.
- **Arguments:**
  - `node` (string, required): Node name.

### `upload_certificate`
- **Description:** Upload a custom SSL certificate and private key.
- **Arguments:**
  - `node` (string, required): Node name.
  - `certificates` (string, required): PEM encoded certificate chain.
  - `key` (string, required): PEM encoded private key.

### `generate_acme_certificate`
- **Description:** Trigger an ACME certificate order/renewal.
- **Arguments:**
  - `node` (string, required): Node name.

## Technical Details
- **API Endpoint:** `/nodes/{node}/certificates`
- **Client:** Update `src/proxmox/system.rs`.
