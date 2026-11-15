---
name: addon-development
description: Keeps addon work in the addon. Use whenever building, extending, porting or fixing a Koldron addon (anything in addons/<id>/, an addon.json, an addon page, or a feature the user calls a plugin or addon), and whenever a core change is made for an addon's sake. Everything that can live in the addon goes there; the core only gets small, generic hooks an addon cannot do without.
---

# Addon development: the addon does the work, the core only opens doors

An addon (`addons/<id>/`, documented in `addons/addons.md`) runs in a sandboxed frame and reaches the app only through the message API. The rule for every addon task: **put as much as possible in the addon, and change the core only where the addon is truly blocked.** Removing the addon's folder must leave the app exactly as it was for users.

## 1. Before writing code

1. Read `addons/addons.md` and the addon's `addon.json`. List what the feature needs: UI places, data, files, network, app data.
2. For each need, check whether the current API already covers it (`packs.*`, `net.fetch`, `files.*`, `folders.*`, `storage.*`, `characters.*`, `ui.*`, the contribution points `settings`, `characters`, `background`, `library`). Use it if it does.
3. For each need it does not cover, decide with the test in section 2. If a core change is needed, **tell the user which hook and why before writing it**, and get approval. Never slip a core change in silently.

## 2. The test for every core change

A change may go in the core only if all of these hold:

- **Blocked:** the addon cannot do it at all from the sandbox. "Slower", "more code" or "nicer" in the addon does not count. Examples of real blocks: a place in the app's UI (a contribution point), the OS (dialogs, files, folders), app data, anything the sandbox forbids (downloads, network without `net.fetch`).
- **Generic:** it is a hook any addon could use, named for what it does (`folders.read`, `library` tab), never for the addon (`pdfs.*`, `localPdfs`). No addon-specific types, formats, file extensions, labels or behaviour in core code.
- **Minimal:** the smallest hook that unblocks the addon. Raw capability, not features: the core lists files and reads bytes; the addon filters, parses, renders, decides. Data conversion (text to bytes, filtering by extension, parsing a format) belongs in the addon.
- **Safe:** permission-checked in Rust against the manifest (a new permission for new powers), never trusting the window. The addon gets ids and contents, never filesystem paths. Reads stay inside what the user granted. Limits on sizes and counts.
- **Needed now:** a real addon uses it in this change. No speculative calls, options or context fields "for later" (e.g. a `?open=` context nothing links to yet).

Things that always stay in the addon: UI and its layout, the feature's data model and storage format (via `storage.*`), parsing and rendering libraries (vendored in the addon folder, e.g. `ui/vendor/`, with their license noted in the addon's `LICENSE.md`), import/export formats, filtering, sorting, searching within its own data, user-facing text of the addon.

## 3. When a core change is approved

Follow the existing pattern end to end, nothing more:

- Rust: manifest field or permission in `src-tauri/src/addons.rs` (validated, with the manifest test updated); the call as an `addon_*` command in `src-tauri/src/commands/addons.rs` with `require(...)` for its permission, registered in `src-tauri/src/lib.rs`; a test for any check that guards security (paths, limits).
- Frontend: a typed wrapper in `src/lib/api/addons.ts`; the call in `src/lib/components/addonBridge.ts` (light argument checks only; permissions are Rust's job); a contribution point where the app shows it, appearing only when an addon uses it.
- Reuse shared helpers instead of copying (`file_inside`, `walk`, `dialog`, `picked_path`, `granted`).
- Docs in the same change: `addons/addons.md` (manifest table, permission table, call table, context) and `docs/SPEC.md` (the Addons section).
- Changing an existing hook's behaviour for one addon is not allowed; extend it generically or add a new one.

## 4. Before calling the work done

1. Run `git diff --stat` and `git diff` on everything outside `addons/`. For **each** core change, write one line: what it is, and why the addon cannot do it. If you can't fill in the why, move that change into the addon or delete it.
2. Grep the core diff for the addon's id, name and domain words (e.g. `pdf`, `ddb`, `beyond`). Any hit outside comments about an example is a leak: move it into the addon.
3. Check the removal test: with the addon's folder gone, does every new hook simply sit unused, with no visible change for users (a contribution point shows nothing, a Settings section is hidden)?
4. Run the type check, the addon's tests (`npx vitest run addons/<id>`), and for Rust changes `cargo test` and `cargo clippy -- -D warnings` in `src-tauri/`.
5. In the summary to the user, list the core changes with their one-line reasons, separately from the addon changes.

Note: `addons/*` is git-ignored except `addons/addons.md`; the core changes are committed, the addon folder is not, unless the user says otherwise.
