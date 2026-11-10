# Changelog

## [0.4.17] - 2026-11-10

### Changed

- All dialogs share one look.

### Fixed

- A copied campaign folder is its own campaign: deleting or changing the copy no longer acts on the original. A folder renamed by hand to a name that can't be a campaign id is listed as unreadable.
- Number fields (HP, AC, initiative, damage amounts and character stats) stop at 99,999, so very large numbers no longer show a technical error.
- Monster pictures load once per session on the encounter screens; offline or with an unsaved picture, the encounter no longer retries on every change, and a picture that can't be kept is reported once (Run no longer hides it).
- A damaged settings file is copied aside once instead of on every read; a pack whose image list names an empty host is rejected.

## [0.4.16] - 2026-11-10

### Added

- Encounters → Run: roll initiative (one roll per monster group, or separately; characters typed in), then step through turns with Next turn (Space) and Previous, rounds and the active turn.
- Run tracks HP (damage takes temp HP first, heal, temp HP), conditions (monster immunities can't be picked), concentration, and death saves for downed characters, with Bloodied, Down, Defeated, Stable and Dead states.
- Run shows the selected monster's full stat block or a character's passive scores, the encounter's notes, an encounter log (automatic and typed entries) and the map with its pins.
- + Combatant during a run: a monster from the packs, a campaign character or a custom one.
- End encounter can write the characters' HP back to them; a running encounter is saved as it changes and offers Continue or Start over when reopened.
- Prepare: split a group of monsters in half, so the same monster can fight as two groups with their own initiative.

### Changed

- The Run buttons on the encounter list and in Prepare are now active.

## [0.4.15] - 2026-11-08

### Changed

- Back is a true back everywhere, the same as the mouse's back button (the Library's Back no longer jumps to the Start screen).
- Library inside a campaign: it opens within the campaign's screens (sidebar and search bar stay), with Back only on a Catalog entry; search results, references and note tags open there while working in the campaign.
- Campaign sidebar: the collapse button is at the top, the campaign settings cog is gone, the collapsed sidebar switches campaigns from the campaign's initial, and collapsing and expanding animate.
- Library: the contents column animates when hidden or shown, and the Settings gear sits at its foot instead of over the chapters.

### Fixed

- Archiving or deleting the open campaign (also from an encounter with unsaved changes) no longer leaves it one Back away.
- Turning packs on or off from any Settings gear refreshes the Library and the search screen.

## [0.4.14] - 2026-11-08

### Added

- Encounter builder: the Party button opens the party in the details card as (compact) character cards, with Party (follows the campaign's party; the default for new encounters) or Custom (click cards to pick); the choice is saved with the encounter.
- Notes: headings in three sizes (Heading 1, 2, 3) and Underline in the toolbar.

### Changed

- Encounter builder: Save is a button with its label; the Notes panel has no title.
- Notes toolbar order: headings, bold, italic, underline, then the lists.
- Encounter cards' party line and difficulty follow the campaign's party for encounters in Party mode; older encounters keep their own pick.

## [0.4.13] - 2026-11-08

### Added

- Characters: a Class filter (a multiclass character shows under each of its classes).
- Characters: cards show current / max HP, the current HP large.

### Changed

- Characters: the party band always shows, with the party's name and the members' names together in its middle, Members and Avg. level (larger, centred) top right, and Add character bottom right.
- Characters: the party's pictures can be two and a half hexagons high, and hexagon collages (also on encounter cards) are more compact.
- Characters: Delete moved from the card's menu (now gone) into the character form, after a confirmation.
- Characters: the name sits at the top beside the picture; the level only shows on its hexagon.
- Characters: the party filter is In party / Not in party, pressed again to turn it off; Sort sits at the right.
- Hover effects ease in and out, and the Characters and Encounters cards glide into place when filtering.

## [0.4.12] - 2026-11-07

### Added

- Characters: the party band shows the party's name, editable in place; it defaults to "<campaign>'s party".

### Changed

- Characters: the party band uses the line colour, with the members' names smaller and centred under the party name.
- Packs: images listed in images.json must be .png.

### Fixed

- Closing the window with unsaved encounter changes now asks Save / Don't save / Cancel instead of losing them.
- Encounter cards ignore deleted characters, so their party line and difficulty stay correct.
- Adding a monster from the browser stops at 999, like the + button.

## [0.4.11] - 2026-11-07

### Changed

- Characters: a new layout. The party sits on a dark band (their pictures as a hexagon collage, always two hexagons high, their names, Members and Avg. level); cards have a large square picture with the level on a hexagon, the menu at the top right and the party button at the bottom right.
- Characters: search also matches the class; filter by All, In party or Not in party; sort by class too.
- Characters: the party button's icon now changes by drawing its lines (users, users-minus, users-plus) instead of fading between icons.

## [0.4.10] - 2026-11-06

### Added

- Encounters → Prepare: Map and Notes buttons under the meter open in the stat block's place. Notes is rich text (bold, italic, a heading, lists).
- Encounter map: add a picture (stored shrunk in the campaign folder, with a progress bar), click it to add numbered pins with a title and rich text in a popup beside the pin, drag pins to move them, and zoom with the scroll wheel (drag to pan).
- Rich text notes: type @ to tag a Catalog entry or one of the campaign's characters; in a Ready encounter a tag opens it.

### Changed

- Encounters → Prepare: the party is chosen with a Characters button under the meter, with the Map and Notes buttons at the bottom of that column; the meter grows to fill it.
- Encounters → Prepare: monster tiles show their CR in the top-left corner instead of type and CR under the name, and are shorter; tighter spacing fits more tiles in a row.
- Encounters → Prepare: buttons, text fields, popups and panels share one style.

### Fixed

- Encounters → Prepare: the Type filter's text no longer runs under the dropdown's arrow.
- Encounters → Prepare: switching between a stat block and a note no longer makes the layout jump.

## [0.4.9] - 2026-11-06

### Changed

- Encounters → Prepare: Run is now a status switch. An encounter starts as Draft; Ready saves and locks it (name, party and monsters can't change), and a pencil puts it back to Draft. Older encounters open as Draft
- Encounters list: a Ready encounter shows a Run button instead of a tag, and each difficulty has its own icon (feather, shield, sword, flame, skull)

## [0.4.8] - 2026-11-06

### Added

- Encounters list: search by name or monster, filter by difficulty, and sort by last edited, name or total XP

### Changed

- The Encounters list shows each encounter as a card: a collage of monster hexagons placed at random on a hex grid (one per monster, the strongest in the middle, the same each visit), Draft or Ready, the monsters, difficulty, XP and party

## [0.4.7] - 2026-11-06

### Added

- Encounters → Prepare: clicking a monster opens its full stat block in a card under the monsters; nothing on the screen goes below the fold, and with a stat block open two rows of monsters still fit.
- Encounters → Prepare: minimize the monsters to one row of pictures (six fit across, each with its count and a ✕), so the stat block gets the rest of the column.

### Changed

- Encounters → Prepare: monsters are compact portrait tiles: a hexagon picture with its ×count, small + and − beside it, the name, type and CR, and an ✕ to remove.
- Encounters → Prepare: Save is an icon button and Start encounter is now Run.
- Panels grow and shrink when minimizing, and stat blocks and popups fade in and out (off when animations are off).
- Thin scrollbars in the theme's colours throughout the app.

## [0.4.6] - 2026-11-06

### Changed

- New look following the Koldron design system: Bone (light) and Cauldron (dark) colours with a potion-green accent, uppercase label-style buttons, table headers and section labels, crisp chips, and borders instead of shadows and glows. Headlines, reading text and numbers are set for Fraunces, Atkinson Hyperlegible and JetBrains Mono; until those fonts are bundled, the system fonts show.

## [0.4.5] - 2026-11-06

### Added

- The window opens at the size and place it had when it was last closed (maximized if it was), unless that place is no longer on a connected screen.

### Changed

- Library: images on Catalog pages show larger (up to 400 px, at most 40% of the page).

## [0.4.4] - 2026-11-06

### Changed

- Library: switching between Rules and Catalog, and opening a chapter or section in Rules, no longer add history steps, so the mouse's back button leaves the Library instead of stepping through what was read.
- Rules: jumping to a section scrolls there with an animation (off when animations are off).
- Catalog: another page opens at its top.

### Fixed

- Catalog: clicking an entry's name no longer reloads the whole app, and going back to the list returns to where it was scrolled.

## [0.4.3] - 2026-11-06

### Changed

- Library: Back from a Catalog entry returns to its list (same category, search, filters, sort and page; the same scroll position when the entry was opened from that list). From anywhere else in the Library, Back goes to the Start screen.

## [0.4.2] - 2026-11-06

### Added

- Settings → Rules: Enable images for the SRD pack. Download all images now (with the estimated size, a progress bar and Cancel) or each one the first time it's shown; they're kept for offline use. Turning it off asks to keep or delete them, and "All N images downloaded" shows when every image is kept.
- Images on Library pages of monsters, magic items and equipment, and in the encounter builder's monster preview.

### Changed

- Encounters → Prepare: the roster is a grid of compact cards (name, type and CR, a quantity field, a red ✕ to remove), animated when monsters are added or removed.
- Encounters → Prepare: hovering a monster previews only its main stats (everything above the traits).

## [0.4.1] - 2026-11-06

### Changed

- Notifications fade out when they close or time out, instead of vanishing at once.

## [0.4.0] - 2026-11-06

### Added

- Encounters: the campaign's encounters, most recently saved first, and Create encounter. The sidebar's Encounters link is now available.
- Encounters → Prepare: a monster browser (search, CR range, type, sortable, 50 per page) with the stat block on hover or focus; a roster with quantities and XP; a party dropdown (the characters in the party start ticked); and a difficulty meter with five bands (Trivial, Easy, Medium, Hard, Deadly) from the packs' 2024 XP budgets. The name is edited in place and saved with Save; leaving with unsaved changes asks Save / Don't save / Cancel.

### Changed

- Characters: "Add to Home" is now "Add to party". A card's bottom-right corner has a party button: users-plus on hover adds the character, and for a character in the party the users icon turns into users-minus on hover to remove it, with a fade.
- The player's name no longer shows on character cards; the total level is shown under the name instead.
- Hover tooltips are removed from every button and link; screen readers keep their labels.

## [0.3.0] - 2026-11-05

### Added

- Campaign sidebar: Home, Library, Characters and Encounters (not yet available), a campaign switcher with a cog that opens Settings → Campaigns, and the Settings gear. It collapses to an icon strip, and becomes a drawer in a narrow window. The search bar sits above every campaign screen.
- Characters screen: a card per character with picture, name, total level and class line, and a home icon when the character is on Home. Search by name or player, sort by name or level. Each card's menu (shown on hover) has Edit, Add to / Remove from Home and Delete.
- Character form: picture, name, player, one or more classes with levels (total at most 20), Max / Current / Temp HP, AC, passive Perception / Insight / Investigation, and Add to Home. A picked picture is dragged (or moved with the arrow keys) into a square; a red ✕ removes it.
- Global search finds characters by name, player or class. Inside a campaign, and in the Library opened from it, search shows only that campaign's characters; the search screen offers Search everything.

## [0.2.0] - 2026-11-05

### Added

- Global search: a search bar on the Start screen, in the Library and on the campaign page. Every word matches, also as the start of a word ("fire ball" finds Fireball), over the switched-on packs' Catalog entries and rule sections.
- While typing, a dropdown of the best 8 results (name, type, source and where it opens); arrow keys move, Enter opens the highlighted result or the search screen, Escape closes.
- Search screen: every result grouped by type, with type filters and counts and a preview of the selected result.

## [0.1.5] - 2026-11-04

### Changed

- The app has a strict Content Security Policy: only its own scripts run, so text from a content pack can never run code.
- All disk work of the Rust commands (campaigns, settings, licenses, packs, rules, catalog) runs off the threads that serve the window.
- Icon buttons share one style; the Settings dialog's close button now looks like the other icon buttons.

## [0.1.4] - 2026-11-04

### Added

- Back returns the Rules reading pane, the Catalog list and entry pages to their scroll position.

### Changed

- One Back button on every screen that has one: a true history back to the previous place, or the Start screen when there is none. Catalog entry pages use the Library's Back instead of their own Back to <category> link.
- Opening a Library tab, category, chapter, section or entry is now a step Back returns from; search, filters, sort and paging change the list in place, so Back from an entry returns to the list with its search and filters.

## [0.1.3] - 2026-11-04

### Added

- Catalog preview cards: a spell's classes, a monster's condition immunities and a background's feat open a card with the entry's facts and text, and Open entry; Escape, ✕ or clicking outside closes it.
- Duplicates are bundled in the Catalog: the same entry in two packs is one row listing both sources, and an entry page shows Also in: with links to the same rule elsewhere (a condition's Rules Glossary term opens in the Rules tab).
- A background entry page (ability scores, feat, proficiencies).
- The Catalog says No rules loaded, with a button to Settings → Rules, when no pack with entries is on.

## [0.1.2] - 2026-11-04

### Added

- Catalog entry pages per type: spells with their casting details and higher-level text; a monster stat block with ability modifiers, defences, senses, CR and its traits and actions; classes with their level table and features; species with their traits.

## [0.1.1] - 2026-11-04

### Added

- Library → Catalog: a Rules | Catalog tab bar, and category tabs (All, Spells, Monsters, Equipment, Magic items, Feats, Classes, Species, Backgrounds, Conditions), each a table with its own columns, sortable both ways.
- Catalog search (every word in the name), filter chips per category with Clear filters, and 50 rows per page; search, filters, sort and page are kept when coming back from an entry.
- Catalog entry pages with the entry's facts and its description.

## [0.1.0] - 2026-11-04

### Added

- Library → Rules: a contents column (chapters with collapsible sections, hideable) and a reading pane showing a whole chapter; clicking a section scrolls to it. Opened from the Start screen's Library button.
- Rules text is rendered from the packs' Markdown with tables; raw HTML shows as text and links and images are left out.
- Settings can open on a given section; the Library's empty state opens Settings → Rules.

### Changed

- Duplicate rules are no longer bundled by the pack loader; bundling moves to the Catalog.
- Packs are read and validated once per run.

### Fixed

- A chapter that lists the same section twice now makes its pack invalid.

## [0.0.8] - 2026-11-04

### Added

- Content packs: a pack.json manifest and pack schema 1, documented in packs/packs.md.
- Pack loader: every pack is validated completely; an invalid pack is listed with its errors and skipped, the others still load.
- Settings → Rules: the installed packs with version, license and entry count, each switchable on or off.
- The SRD pack's pack.json and encounter budgets (XP per character by level), built by scripts/build-srd-pack.mjs from the source data, which is used in place.
- Duplicate rules (same name, at least 80% the same text) are bundled automatically, without removing any from the data.

### Changed

- The duplicate copy of the 5e-database MIT license in packs/srd/source/ is removed; packs/srd/LICENSE.md keeps the full text.

## [0.0.7] - 2026-11-04

### Added

- SRD rules text: the rules chapters of the SRD 5.2.1 (Playing the Game, Character Creation, Spellcasting, Rules Glossary with each term as its own entry, Gameplay Toolbox, and the Magic Items and Monsters intros) as 5e-SRD-Rules.json and 5e-SRD-Rule-Sections.json in the pack source.

### Changed

- The SRD pack license now also covers 5e-SRD-Rule-Sections.json; the SRD source notes moved out of the pack folder.

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
