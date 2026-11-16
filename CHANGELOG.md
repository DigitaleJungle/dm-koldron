# Changelog

## [0.6.6] - 2026-11-16

### Added

- Addons can keep their own files (storage.putFile, getFile, deleteFile, listFiles)
- Addons can show a file of a picked folder in the system file manager (folders.reveal)
- An addon's Library tab can add steps to the app's history (ui.navigate), so Back and Forward move through it
- An addon can hide or show its own Library tab (library.setVisible), remembered across restarts

### Changed

- Addon docs: heavy work runs in a classic Web Worker started from a blob URL; module workers are refused in the sandbox

## [0.6.5] - 2026-11-15

### Added

- Addons can add a tab to the Library (library in addon.json), shown after Catalog
- Addons can add a panel to a new Settings → Library section, shown only when an addon uses it
- Addons can read folders the user picks (folders:read permission: folders.add, list, remove, scan and read); the addon never sees where a folder is, and reads stay inside it
- Addons can save a file where the user picks (files.saveFile)

### Changed

- An addon's Library tab is closed while Settings is open and loads again when Settings closes

## [0.6.4] - 2026-11-15

### Changed

- Addons declare which character fields they keep up to date (characters.owns in addon.json); the app locks exactly those fields in the character form and in a running encounter, and everything else stays editable
- A linked character's name and player can be changed in the app when its addon doesn't own them

## [0.6.3] - 2026-11-15

### Changed

- A character linked to an addon that isn't installed behaves like any other (editable, normal HP in encounters); putting the addon back links it again

### Fixed

- Addon pages: forms work (submitting a form in an addon did nothing)
- Saving a character no longer fails when the addons folder can't be read

## [0.6.2] - 2026-11-15

### Added

- Addons can keep characters up to date: a characters API (characters:write), an item in the Characters screen's Add character menu (a menu only when an addon adds one), a hidden background page, and short notifications shown after the addon's name
- Linked characters: a character an addon keeps up to date shows where it comes from with Edit link; its name, classes and stats can't be changed in the app, its player, picture and party can; death saves are kept with it

### Changed

- Running an encounter: a linked character's HP, max and temp HP and death saves follow what its addon syncs, its HP controls are disabled, and End encounter doesn't write its HP back

## [0.6.1] - 2026-11-15

### Changed

- The app creates the empty addons folder in its data folder when it starts, so there's a place to put addons from the first launch on

## [0.6.0] - 2026-11-15

### Added

- Addons: code in the data folder's addons/<id>/ runs in a sandboxed frame and reaches the app only through a permissioned message API (addons/addons.md); a fresh install has none
- Addons can list, install and remove packs; an installed pack is fully validated before an atomic swap, and one with a bundled pack's id (an updated SRD) is used instead of it until removed
- Settings → Rules shows the panel of each addon that adds one; Settings → About lists the addons, invalid ones with their errors, and their license texts

## [0.5.18] - 2026-11-14

### Added

- Campaign Home: widgets for the party, the next session, the previous session's summary and random encounters, in a three-column grid; drag a grip to move a widget and its right edge to resize it, or use its ⋯ menu
- Campaign Home: a lock that fixes the layout and hides the widgets' cards; while unlocked, Remove in a widget's ⋯ menu takes it off Home and a dashed Add widget slot puts it back
- Campaign Home: add or remove party members from the Party widget; the encounters' difficulty follows at once
- Campaign Home: the Random encounter widget shows one encounter per column of its width, each with a die to pick another
- Encounter builder: Delete next to Back removes a saved encounter with its map and run, and unlinks it from session scenes

### Changed

- Encounters: with no encounters yet, a dashed Create encounter card shows instead of a message, on the Encounters screen and on Home

## [0.5.17] - 2026-11-13

### Added

- Run session: the live view for a session at the table, with the party's passives and HP, a run sheet, the current scene in Info, Location and NPC tabs, scene and session checklists, and DM notes
- Run session: mark scenes played in any order, pause and resume the timer, and end the session with a summary written beside the DM's notes
- A Played session can be run again from Session prep, and its summary edited from the Sessions overview

### Changed

- Session prep: Run session is enabled, and the Summary field moved to End session

## [0.5.16] - 2026-11-12

### Changed

- Inside a campaign, the search bar is centred and larger, and Back sits in its header
- Encounter builder: Save, Ready, Back to draft and Run moved to the header; the encounter name sits above the monster search
- Session prep: Delete sits next to Back and Run session at the top right of the header
- Library Rules inside a campaign: the contents column is always shown (no collapse button)

## [0.5.15] - 2026-11-12

### Changed

- Session prep: a scene's Links tab shows cards: the location (banner, where it sits, its atmosphere), each NPC (portrait and details) and each encounter (its Encounters card without Start and XP); a pencil opens a location or NPC in its editor, a cross unlinks.
- Session prep: Locations, NPCs and Encounters show how many the session's scenes link ("2 added") beside their names.
- Encounter cards show how many monsters instead of listing them.

### Fixed

- Session prep: closing a location or NPC editor returns to the scene's open tab, with the focus back on its pencil.

## [0.5.14] - 2026-11-12

### Changed

- Session prep: Locations, NPCs and Encounters moved to the bottom left, under the scenes; an open one grows up over the scenes, which scroll under a pinned Scenes heading.
- Session prep: Locations and NPCs show the campaign's trees as on the Sessions overview, with Add beside the section's arrow; the Manage locations and Manage NPCs screens are gone.
- Session prep: Encounters has a search and matching rows (name and difficulty), with Manage encounters and a + for a new encounter beside the arrow.
- Session prep: sections open with a growing animation; the Not written yet and checklist done labels are gone.
- Removing a scene asks with a check above a cross in the trash can's place.

## [0.5.13] - 2026-11-12

### Added

- Drag rows in the Locations and NPCs trees to move them: a location into another, an NPC or folder into a folder, or to the top
- A folder's rename row has an Inside dropdown to move it from the keyboard

### Changed

- Clicking a folder's name opens or closes it; Add puts new NPCs and folders in the folder last clicked
- A folder's trash can comes before its pencil
- Richer hover cards: an NPC's portrait, details and stats; a location's image, atmosphere, purpose and linked NPCs

### Fixed

- Text in the hover cards was centred and squeezed

## [0.5.12] - 2026-11-12

### Added

- NPC fields in tabs: Profile (race, gender, age, job and alignment in a table, a portrait, stats and a summary), Motivation and traits, and Backstory
- NPC stats in a stat-block box (AC, HP, speed, CR and ability scores), copied from a catalog creature (Commoner by default) or rolled with dice, with Ctrl+Z to undo
- NPC groups: folders in the NPCs tree that nest, added and renamed in the tree

### Changed

- NPCs no longer sit inside each other; NPCs saved inside another NPC move to the top level, and Wants and Voice become Motivation and Traits
- Manage NPCs is a list that opens the NPC editor, with groups as headings
- The overview's hover card and editor no longer show which sessions an entry appears in

## [0.5.11] - 2026-11-11

### Added

- Locations have Atmosphere, Purpose and Secrets (rich text), images (add, remove, open larger with previous and next), a map with pins like an encounter's, and linked NPCs.
- An NPC can be dragged onto the location being edited to link it: the editor gets the session prep's dashed outline and tint, and Linked NPCs opens as soon as the NPC is picked up.

### Changed

- The location editor uses the scene editor's tabs (Atmosphere, purpose and secrets; Images; Map; Linked NPCs) and fills its column; the three text fields share the height and only scroll when their text is longer.
- A location's earlier description is now its Atmosphere.
- Manage locations on the session page lists the locations; one opens in the full location editor, and a location made from a scene opens there.
- Pictures a location no longer uses are removed from the campaign folder when it next loads.

## [0.5.10] - 2026-11-11

### Changed

- Locations and NPCs on the Sessions overview are no longer edited in their column: an eye on each row shows a card with its details (on hover, focus or click), and a pencil opens it in an editor in place of the sessions, closed with a cross.
- The card sits beside the eye, never over the rows above or below, and fades in and out.
- Adding a location or NPC on the overview opens it in the editor.
- The rows of the Locations and NPCs columns start at the panel's left edge; the counts beside them are gone.
- The other unplayed sessions follow Up next without a Later heading.

### Fixed

- Link buttons on the Sessions overview (Continue prep, Open, New session) are no longer underlined.

## [0.5.9] - 2026-11-11

### Added

- Sessions overview: the campaign header (party and average level, played sessions, locations and NPCs), Up next with its premise and counts, Later, Planned when nothing is left to play, and Played sessions with their summary.
- Session status: Drafting, Ready, Playing and Played.
- Locations and NPCs trees on the Sessions overview, with search, adding and editing in place; selecting one highlights the sessions it appears in.
- A Summary for each session, in the session's right column.

### Changed

- Locations and NPCs belong to the campaign instead of a single session, and can be nested; Manage locations and Manage NPCs edit them, with an Inside field.
- Existing sessions' locations and NPCs move to the campaign the first time its sessions open, same names merged into one; the old session files are backed up first.
- Removing a location or NPC asks first, and what was under it moves up a level.

## [0.5.8] - 2026-11-11

### Changed

- A Ready scene shows What happens as rendered text instead of a locked field; its tables keep their column widths and have banded rows, a shaded header row and no borders, and boxed text has its own background.
- Hovering or focusing a tag in a Ready scene's text shows a preview card of the entry or character.
- The open scene is split into three tabs (What happens; location, NPCs and encounters; checklist), stacked as hexagons along the panel's left edge, with Remove scene as a trash can at the bottom.
- What happens fills the panel's height.
- Dragging a card shows a dashed outline around the open scene and the scenes in the list, easing in and out, instead of a banner.
- Focus rings show only when moving with Tab, not after a click or tap.
- Shown notes lay tables out with the column widths set in the editor.

### Fixed

- A wide table in a Ready scene scrolls on its own instead of moving the whole panel.

## [0.5.7] - 2026-11-11

### Changed

- The session header puts small dots between the date, the start time and the player count.
- Scenes in the scene list show icons for what they have (What happens written, NPCs, a location, encounters) instead of a line of text.
- A Ready scene is locked: its title, What happens and adding or removing checklist items can't change until Ready is switched off.

### Fixed

- Hovering the selected scene in dark mode no longer makes its text unreadable.
- A Ready scene's hexagon is visible in dark mode.

## [0.5.6] - 2026-11-11

### Added

- Sessions: What happens has a Text style dropdown (Paragraph, Heading 1 to 3) in place of the heading buttons, and Align left, centre and right.

### Changed

- Rich text: a tool that can't apply where the cursor is (a heading or an alignment in a table cell) is greyed out.
- Sessions: the What happens label is gone.

### Fixed

- Rich text: paragraph and table spacing, and table cell padding, apply on every screen, not only after opening the Library, Search or Encounters.

## [0.5.5] - 2026-11-11

### Added

- Rich text: tables can be resized (drag a column's edge; double-click it for automatic width), and have header row and column toggles and merging or splitting cells, all in a Table dropdown while the cursor is in a table.

### Changed

- Rich text: every rich text field (encounter notes, map pins, What happens) now uses the Tiptap editor; undo covers everything, and a tag is removed whole with Backspace.
- Rich text: the highlighted section is now boxed text, set apart by a bar on its left.
- Rich text: a new paragraph leaves a full blank line; table cells hold text and line breaks, and a table sits right under the text before it with a blank line below it.

### Fixed

- Rich text: merged cells and column widths are kept when a note is saved.

## [0.5.4] - 2026-11-11

### Added

- Sessions: What happens is a rich text field (headings, bold, italic, underline, lists, @ tags) with a highlighted section and tables (add or remove rows and columns).

### Changed

- Sessions: scenes are reordered by dragging them in the list, or with their grip and the arrow keys; the up and down buttons are gone.
- Sessions: Ready is a check icon beside the scene title, Remove scene moved to the bottom of the scene, and the Scene x of y label is gone.
- Sessions: a scene's location, NPCs and encounters share one compact block; the location's description shows under it and is edited in Locations (+ New location opens Locations to name it).
- Sessions: the party note moved from the header to the Across session column.

### Fixed

- Sessions: the selected scene's highlight runs to the end of its row, under the grip.

## [0.5.3] - 2026-11-11

### Changed

- Sessions: session prep rebuilt around scenes. Each scene has what happens, a location, linked NPCs, linked encounters, its own checklist and a Ready state; locations and NPCs are shared across the session, with Previously, Premise and a session checklist in an Across session column (one section open at a time).
- Sessions: location, NPC and encounter cards can be dragged onto a scene; the scene's dropdowns do the same from the keyboard. Encounters lists the campaign's encounters that are ready to run, and Manage encounters opens the Encounters screen.
- Sessions: changes save automatically; leaving after a failed save asks Stay or Leave anyway. New session creates the session at once.
- Sessions: the header shows the party's hexagon collage, a party note, counts for scenes, NPCs and checklist items, and a Run session button (disabled for now).
- Sessions: two sessions can no longer have the same number.
- Sessions: the Note, List, Encounter and Map steps, To bring and Open threads are gone; sessions saved in the step format open without their steps.

## [0.5.2] - 2026-11-10

### Added

- Sessions: a Map step, the same map as in encounters (a picture with numbered pins, each with a title and text; zoom and pan). Choosing a picture for an unsaved step saves the session first; the picture is removed with its step or session.

## [0.5.1] - 2026-11-10

### Changed

- Campaign sidebar: each screen can be moved with a grip shown on hover or focus (drag it, or use the up and down arrow keys); the order is the same for every campaign, remembered on this device, and the icon strip follows it.

## [0.5.0] - 2026-11-10

### Added

- Sessions screen in the campaign sidebar: a list of sessions (number, title, date and time, prep progress) with search and New session.
- Session prep page: steps you add yourself (Note with rich text and @ tags; List as bulleted, numbered or checklist; one Encounter, new or picked, with Edit in builder and Run encounter), each marked ready, with the prep progress in the header.
- Session page right column: the party with a note for the session, the campaign-wide Open threads (saved at once) and a To bring list.

## [0.4.18] - 2026-11-10

### Added

- Characters have an Initiative modifier (can be negative); it shows in search and on the Roll initiative step of Run, and breaks initiative ties.
- + Combatant: a monster can be given its initiative instead of rolling it (left empty, it still rolls).

### Changed

- Run: the Amount field and the Damage, Heal and Temp HP buttons are more compact.
- Encounters: the difficulty filter has no All button; pressing the pressed band again shows all encounters, as on the Characters screen.

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
