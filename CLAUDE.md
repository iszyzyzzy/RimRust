# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common commands

Frontend (pnpm + Vite + Vue 3):
- Dev (frontend only): pnpm dev
- Dev (full app via Tauri): pnpm tauri dev
- Build frontend: pnpm build
- Preview built frontend: pnpm preview

Tauri backend (Rust):
- Build Rust crate: cargo build
- Run full app (calls frontend dev/build as configured): pnpm tauri dev / pnpm tauri build

Tests:
- Frontend: No test framework configured in package.json; no *.test / *.spec files found
- Rust: cargo test
- Single Rust test: cargo test <test_name>

Notes:
- Tauri build/dev hooks: src-tauri/tauri.conf.json → beforeDevCommand: pnpm dev, beforeBuildCommand: pnpm build
- Type-checking is enforced in pnpm build via vue-tsc --noEmit

## Architecture overview

This is a desktop app built with Vue 3 (Vite) frontend and a Rust (Tauri) backend.

Frontend:
- Entry: src/main.ts mounts src/App.vue
- State: Pinia (stores under @store alias → src/components/utils/store/)
- UI: Naive UI via auto-import; components under src/components/**
- Auto-imports: unplugin-auto-import and unplugin-vue-components configured in vite.config.ts; type shims in auto-imports.d.ts and components.d.ts
- IPC: Frontend calls Rust commands via @tauri-apps/api/core invoke and listens to events via @tauri-apps/api/event. See src/api/tauriFunc.ts for wrappers.
- App bootstrap flow (src/App.vue: lines ~21-75):
  - On mount, listenStartLoading and listenEndLoading update a loading modal
  - Query save metadata (loadSaveMetaData) and conditionally call init_mission in Rust (invoke)
  - After init, fetch BaseList once, then subscribe to sync-many events to keep state in sync

Backend (Rust, src-tauri/src/*):
- Entry: src-tauri/src/main.rs calls rimrust_lib::run()
- Core: src-tauri/src/lib.rs orchestrates Tauri Builder, plugins, logging (tracing), app state, and command registration
- Commands: #[tauri::command] functions exported and registered via generate_handler! in lib.rs; primary API surface in src-tauri/src/func.rs
- App state: AppConfig struct (lib.rs:240+) managed as a tauri::State<Mutex<AppConfig>>; saved/restored from app_data_dir/app_config.json
- Background tasks: src-tauri/src/background_task.rs provides TaskManager; tasks scheduled in init_mission (lib.rs:403+)
- File system and watchers: src-tauri/src/file/* includes reader.rs (load missions), watcher.rs (file watch), xml.rs
- Mods domain: src-tauri/src/mods/*
  - base_list.rs: core mod list, autosave, sync-many payload handling
  - sort.rs: mod load order logic
  - scan.rs: error/conflict scanning
  - search/*: search engine (basic/advance); tokenizer.rs includes a Rust test module
  - translate/*: translation matching and management
  - community_data/*: community rules and Steam DB updates
  - steam.rs, xml.rs: integrations and parsing
- Types: src-tauri/src/types/* (common types, priority mutex, pre_define) and frontend types in src/api/types.ts

Events and data flow:
- Rust emits start-loading/end-loading via tauri::Emitter; App.vue listens and toggles UI
- init_mission schedules background updates (community rules, Steam DB) and generates load missions for mods/data paths; BaseList is managed on the app state and autosaves

## Configs and aliases
- vite.config.ts: aliases @, @assets, @api, @components, @utils, @store; server fixed port 1420 for Tauri; HMR configured via TAURI_DEV_HOST; ignores src-tauri during Vite watch
- tsconfig.json: strict TS, noEmit, path aliases matching Vite; includes Vue SFCs
- src-tauri/tauri.conf.json: productName, identifier, window sizing; build.beforeDevCommand pnpm dev; build.beforeBuildCommand pnpm build; devUrl http://localhost:1420; CSP and asset protocol enabled
- Cargo.toml: crate name rimrust_lib; plugins: opener, dialog, log, process, single-instance, devtools (debug-only pattern in lib.rs)

## Important repository rules (from .github/copilot-instructions.md)
- Do not operate in or generate code under .history/ and src-tauri/target/
- Respect Rust module boundaries: file/ for filesystem ops, mods/ for mod logic, types/ for common types
- Vue components should prefer <script setup> and reusable components live under src/components/utils/components/
- Development entry points: pnpm tauri dev for full-stack dev; pnpm tauri build for packaging

## Pointers for future changes
- Use src/api/tauriFunc.ts for frontend-to-backend calls and event listeners
- If working with mod data, BaseList and related types/functions are under src-tauri/src/mods/**
- Logging setup and panic hook are configured in lib.rs; follow the existing tracing approach if you need to inspect behavior during development