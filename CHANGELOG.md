# Changelog

## [0.0.6] - 2026-11-04

### Added

- Settings → About: app version, licenses, attributions (including the SRD 5.2.1 statement) and the full license texts, read from the app's and each pack's license file.

### Changed

- The SRD pack license and the README now state that the SRD material was converted into the DM's Koldron pack format; the SRD pack license includes the 5e-srd-api MIT notice.
- Removed a trademark from the app's package description.

## [0.0.5] - 2026-11-04

### Added

- Start screen: campaign tiles (most recently opened first), + New campaign, and the Settings gear and version; the Library button and search bar are placeholders for now.
- Settings dialog: UI (Appearance and Animations, saved app-wide and applied from the first paint) and Campaigns (add, rename inline, picture, archive / restore, delete permanently after a confirmation).
- Notifications for campaign actions and errors.
- Campaign storage: one folder per campaign with a unique internal id, written atomically; campaign pictures are shrunk and stored in the campaign folder.

### Fixed

- Icons now show in development (the icon font folder is served), and the filled icon stylesheet no longer overrides the outline icons.
- A damaged settings file no longer blocks saving UI settings; a copy of it is kept.

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
