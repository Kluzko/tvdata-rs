# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog and the project follows Semantic Versioning.

## [Unreleased]

### Changed

- documented MSRV, stability policy, and public API contract expectations for downstream users and contributors

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
