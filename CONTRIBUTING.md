# Contributing

Thanks for contributing to `tvdata-rs`.

The project is intentionally library-first: typed APIs, predictable behavior, and clean module boundaries matter more than adding surface area quickly.

## Development Setup

- Rust `1.85+`
- `cargo`
- network access if you want to run live examples against TradingView

Clone the repository and work from the crate root.

## Development Workflow

Before opening a change, run the standard validation set:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test -q
cargo check --examples -q
```

## Project Guidelines

- Keep the public API typed and predictable.
- Prefer high-level product models over leaking raw TradingView payloads.
- Keep low-level TradingView field ownership under `src/scanner/fields/`.
- Prefer thin facade modules and move heavy tests into sibling `tests.rs` files when modules grow.
- Do not add built-in storage or database layers to the crate core.
- Do not silently weaken validation just to make a request pass.

## Docs

If you change public APIs, update:

- `README.md`
- `README.snippet.md`
- relevant rustdoc examples
- `examples/` when the change affects user-facing workflows
- `CHANGELOG.md` when the change affects the public contract

## Versioning And Stability

- Current MSRV is Rust `1.85`.
- Patch releases should avoid intentional public API breakage.
- While the crate is still `0.x`, minor releases may contain breaking changes, but they should be explicit and documented.
- Public, documented crate exports are treated as the stability contract. Internal transport details and non-public modules are not.

If you introduce a breaking public change, add a clear changelog entry and any migration notes needed for downstream users.

## Testing Notes

Unit tests should stay deterministic and not depend on live TradingView responses.

Live checks are useful, but they should stay in examples or ad hoc validation, not in the normal test suite.

## Scope

Good contributions usually improve one of these areas:

- correctness
- type safety
- docs and onboarding
- capability awareness
- transport resilience
- layout and maintainability

Large feature additions should preserve the crate's existing design: a clean Rust library, not a service framework.
