# account-vault-app

Desktop app for storing account credentials (usernames, passwords, emails, etc.) with encrypted data export/import.

Built with [Tauri 2](https://tauri.app/) (Rust backend) and React + TypeScript (Vite).

## Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [Rust](https://rustup.rs/) (stable)
- Windows: Microsoft C++ Build Tools and WebView2 (preinstalled on Windows 11)
- Other OSes: see https://tauri.app/start/prerequisites/

## Development

```sh
npm install
npm run tauri dev
```

## Build

```sh
npm run tauri build
```

The installer is written to `src-tauri/target/release/bundle/`.

## Project layout

- `src/` — React frontend (UI)
- `src-tauri/` — Rust backend (vault storage, encryption, import/export)
