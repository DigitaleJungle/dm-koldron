# Koldron content packs (schema 1)

A content pack is a folder with a `pack.json` manifest and JSON data files. DM's Koldron loads every folder in `packs/` that has a `pack.json`; other folders are ignored. The SRD pack (`srd/`) is loaded the same way as any other pack.

## pack.json

```json
{
  "schema": 1,
  "id": "my-homebrew",
  "name": "My Homebrew",
  "version": "1.0.0",
  "license": "CC-BY-4.0",
  "attribution": "Text the license requires you to show, or empty.",
  "ruleset": "2024",
  "files": [
    { "type": "spells", "path": "data/spells.json" },
    { "type": "rules", "path": "data/rules.json" },
    { "type": "rule-sections", "path": "data/rule-sections.json" }
  ]
}
```

Every field is required, and fields not listed here are an error.

| Field | Rule |
|---|---|
| `schema` | `1`. Packs with another schema aren't loaded. |
| `id` | Lowercase letters, digits and dashes; unique among the installed packs. |
| `name`, `version`, `license`, `ruleset` | Not empty. `ruleset` names the rules the pack is for, e.g. `2024`. |
| `attribution` | A string, empty if no attribution is needed. |
| `files` | At least one. `path` is relative to the pack folder and can't leave it. Several files may have the same type. |

## Data files

Each file is a JSON array of objects. Entries use the field names of [5e-database](https://github.com/5e-bits/5e-srd-api/tree/main/packages/5e-database) (2024 data). Every entry has a non-empty `index`, unique within its type in the pack, and a non-empty `name`.

Types: `ability-scores`, `alignments`, `backgrounds`, `classes`, `conditions`, `damage-types`, `encounter-budgets`, `equipment`, `equipment-categories`, `feats`, `features`, `languages`, `levels`, `magic-items`, `magic-schools`, `monsters`, `poisons`, `proficiencies`, `rule-sections`, `rules`, `skills`, `species`, `spells`, `subclasses`, `subspecies`, `traits`, `weapon-mastery-properties`, `weapon-properties`.

Extra checks:

- **`monsters`**: `challenge_rating` and `xp` are numbers, or left out (or null) for a creature without a challenge rating, such as a summoned one; it then shows "—" and counts as unknown XP in encounters.
- **`rules`** (chapters): `index`, `name`, `desc` (Markdown, the chapter's intro) and `subsections`, a list of `{ "index": "…" }` naming `rule-sections` entries **of the same pack**. A pack adds its own chapters only; it can't add sections to another pack's chapter.
- **`rule-sections`**: `index`, `name` and `desc` (Markdown).
- **`encounter-budgets`**: XP per character by level, no `index` or `name`: `{ "level": 1, "low": 50, "moderate": 75, "high": 100 }`, with whole numbers and each level once.

Markdown is CommonMark with tables. The reader shows the chapter and section names itself, so a `desc` that starts with a heading repeating the name has that heading left out. Raw HTML is shown as text, and links and images are left out (their text stays), except a **tag**: a link to `koldron:<type>/<pack>/<index>` (each part letters, digits, `_`, `.` or `-`), e.g. `[Fireball](koldron:spells/srd/fireball)`, which opens that entry and shows its summary on hover. A tag to an entry that isn't loaded opens the entry from the first switched-on pack that has its index, or shows that it isn't found.

## images.json (optional)

The images the pack's entries name (an entry's `image`, e.g. `/api/images/monsters/aboleth.png`) that can be downloaded when the user turns on **Enable images** for the pack in Settings → Rules. Images are never shipped in a pack. A pack without `images.json` can instead show images that the addon which installed it keeps for it (`packs.putImages` in `addons/addons.md`).

```json
{ "host": "https://www.dnd5eapi.co", "images": { "/api/images/monsters/aboleth.png": 757000 } }
```

`host` is `https://` and a host name only. Each key starts with `/`, ends in `.png` and is fetched from `host`; its value is the download size in bytes, for the estimate the app shows. The app only downloads paths listed here. For the SRD pack, `cargo run --example image_manifest` (in `src-tauri/`) writes this file from the data, leaving out images the server doesn't have.

## Validation

A pack is checked completely before any of it is used. A pack with any error isn't loaded at all; Settings → Rules lists it with its errors, and the other packs still load.
