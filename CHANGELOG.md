# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and the project follows Semantic Versioning.

## [Unreleased]

## [0.1.2] - 2026-03-23

### Changed

- backend history presets now use a tuned chart-history envelope with `6` concurrent websocket sessions and matching history batch concurrency
- history batch execution now caps effective concurrency to the configured websocket request budget
- auto snapshot batching now keeps current 1000-symbol public snapshot surfaces on a single request, while still slicing larger payloads
- `daily_bars_on` now reuses chart websocket sessions across symbol chunks with sequential `modify_series` batching, small-first windows, and progressive expansion instead of opening one socket per symbol

### Added

- regression coverage for websocket-capped history batching and backend-history daily ingestion envelopes
- configurable snapshot batch planning with `Auto`, `SingleRequest`, and explicit chunked modes
- a live batch benchmark example for comparing snapshot and `daily_bars_on` throughput envelopes, including configurable history concurrency lists

## [0.1.1] - 2026-03-22

### Added

- backend-oriented history helpers for daily bar selection, partial batch results, and richer symbol-level failures
- grouped client configuration via `TradingViewClientConfig` and `TransportConfig`
- request-budget controls for HTTP pacing, HTTP concurrency caps, and websocket session caps
- injectable websocket connectors for history and quote-session flows
- typed observer hooks for HTTP, websocket, and history-batch events
- tracked offline regression fixtures for search, scanner, economics, and history payloads

### Changed

- public onboarding docs now reflect the current initialization model, including grouped config, presets, budgets, connector injection, and observer hooks
- documented MSRV, stability policy, and public API contract expectations for downstream users and contributors
- improved backend integration ergonomics around auth, presets, transport ownership, and operational visibility

## [0.1.0] - 2026-03-22

Initial public crate release.

### Added

- low-level TradingView scanner support with typed query building
- live scanner metainfo and capability-aware validation
- high-level `equity`, `crypto`, and `forex` facades
- OHLCV history via TradingView chart websockets
- rich `symbol_search/v3` support
- macro economic calendar support
- market-wide earnings, dividend, and IPO calendars
- equity analyst summaries, estimate history, and point-in-time fundamentals
- typed client configuration with retry and endpoint overrides
- auth-aware `sessionid` support for HTTP and websocket requests

### Changed

- codebase layout was modularized into clearer folder-based modules
- public documentation was expanded for first-time users
- scanner field ownership was centralized under `src/scanner/fields/`
