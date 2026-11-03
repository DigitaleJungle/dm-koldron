# Changelog

## [0.0.4] - 2026-11-04

### Changed

- README: removed the Wizards of the Coast disclaimer line; the SRD attribution statement will be the only mention of Wizards.
- Project guidance: a new build order, kept in a local TODO list.

## [0.0.3] - 2026-11-03

### Added

- App scaffold: Tauri 2 + SvelteKit (Svelte 5, TypeScript) with a placeholder start screen, design tokens and UI strings.
- Separate dev identifier so dev builds keep their own data folder; a debug build without it refuses to start.
- Tabler Icons 3.50.0 in `packs/tabler-icons/` (MIT).
- Version script, pre-commit hook and `npm run verify` (type check, tests, clippy, cargo tests).

## [0.0.2] - 2026-11-03

### Added
- SRD 5.2.1 source data in `packs/srd/source/`: the 2024 English data from 5e-srd-api (5e-database v7.9.1), with its license and a record of where it came from.
- License and attribution for the SRD pack (`packs/srd/LICENSE.md`).

### Changed
- Project guidance: the SRD pack is built only from the data in this repo, from 5e-srd-api instead of the archived 5e-database repo, with the SRD's own attribution statement.

## [0.0.1] - 2026-11-03

### Added
- Project guidance for Claude Code (CLAUDE.md): product scope, stack, content packs, domain and UX rules.

## [0.0.0] - 2026-11-03

### Added
- Initial repository: license, README, and the commit and publish workflows.
