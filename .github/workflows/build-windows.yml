name: Windows-Build

on:
  push:
    branches: [ main, master ]
  workflow_dispatch:

jobs:
  build-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - name: Rust-Toolchain installieren
        uses: dtolnay/rust-toolchain@stable

      - name: Release bauen
        run: cargo build --release

      - name: exe als Artefakt hochladen
        uses: actions/upload-artifact@v4
        with:
          name: emble_gui_rs-windows
          path: target/release/emble_gui_rs.exe
