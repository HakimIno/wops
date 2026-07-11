# wops

Linux-first screen recording and streaming application built with Rust, egui, wgpu, and PipeWire.

## Fedora development

Install the native packages used by the capture backend:

```bash
sudo dnf install pipewire-devel clang
```

If system packages cannot be installed, bootstrap a workspace-local PipeWire SDK instead:

```bash
./scripts/bootstrap-fedora.sh
```

Build and run:

```bash
cargo test --workspace
cargo run -p wops-app
```
