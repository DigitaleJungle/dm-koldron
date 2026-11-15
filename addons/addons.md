# Koldron addons

An addon is code that extends DM's Koldron. It runs in a sandboxed frame inside the app and talks to it through a small message API. It can't reach the app's window, your files or the network except through that API, and only as far as the permissions in its manifest allow.

Content packs are not addons (see `packs/packs.md`). An addon can produce packs, for example an importer.

## Installing

There's no install screen yet. Put the addon's folder in the app's data folder, `addons/<id>/`, and restart the app. The app creates the empty `addons` folder when it starts:

- Windows: `%APPDATA%\be.digitalejungle.koldron\addons\`
- macOS: `~/Library/Application Support/be.digitalejungle.koldron/addons/`

A development build uses `be.digitalejungle.koldron.dev` instead. Settings → About lists the addons, and an invalid one with its errors.

Removing the folder removes the addon. Data it saved (`addon-data/<id>/`) and packs it installed (`packs/<id>/`) stay.

## The folder

```
my-importer/
  addon.json        the manifest (required)
  LICENSE.md        shown in Settings → About (or LICENSE, LICENSE.txt)
  ui/settings.html  a page, and whatever scripts, styles and images it loads
```

File names: plain ASCII, no spaces.

## addon.json

```json
{
  "api": 1,
  "id": "my-importer",
  "name": "My importer",
  "version": "1.0.0",
  "license": "MIT",
  "permissions": ["packs:read", "packs:write", "net:api.github.com"],
  "settings": { "section": "rules", "entry": "ui/settings.html" },
  "characters": { "label": "D&D Beyond", "entry": "ui/add.html" },
  "background": "ui/background.html"
}
```

| Field | |
|---|---|
| `api` | The addon API version it's written for. This app speaks `1`; an addon for another version isn't loaded. |
| `id` | Lowercase letters, digits and dashes; the same as the folder name. |
| `name`, `version`, `license` | Required, shown in Settings → About. |
| `permissions` | Optional. See below. Unknown permissions make the addon invalid. |
| `settings` | Optional. A page shown as a panel in a Settings section. Only `rules` (below the pack list) for now. |
| `characters` | Optional. An item labelled `label` (at most 40 characters) in the Characters screen's **Add character** menu, opening the page in a dialog. Add character is a menu only when an addon adds an item. The same page opens from **Edit link** on the form of a character linked to the addon. |
| `background` | Optional. A page that runs hidden for as long as the app is open. |

### Permissions

| Permission | Allows |
|---|---|
| `packs:read` | `packs.list` |
| `packs:write` | `packs.install`, `packs.remove` |
| `characters:write` | `characters.list`, `characters.save`, `characters.setPicture` |
| `net:<host>` | `net.fetch` to `https://<host>/…`, one permission per host name (no port, no wildcards). Only the address the addon asks for is checked; redirects the host sends are followed, wherever they lead (GitHub release downloads need them). |

Placing the folder is the consent: an addon gets exactly what it declares, and every other call is refused.

## The page

Pages are served from `koldron-addon://localhost/<id>/…` (on Windows `http://koldron-addon.localhost/<id>/…`). They run with an opaque origin, so there's no cookies, `localStorage` or IndexedDB: use `storage.*` instead. Their Content Security Policy allows scripts, styles, images, fonts, workers and `fetch` from the addon's own files only (plus `data:`/`blob:` images and `blob:` workers). There's no other network access: use `net.fetch`.

Forms work as usual: listen for `submit` and call `preventDefault()`. A form can't be sent anywhere (the policy has `form-action 'none'`).

Run heavy work (unzipping, converting) in a Web Worker so the app's window stays responsive.

### Context

The app tells a page where it is with a message (not a call, so there's no answer): `{ koldron: 1, event: "context", context }`. It's sent once the page has loaded and again whenever the context changes.

- Background page: `{ campaign, live, screen }`. `campaign` is the open campaign's id, or null. `live` is true while a running encounter or a running session is shown. `screen` is the path of the screen shown, so a new context arrives every time a screen loads.
- `characters` page: `{ campaign, character }`. `character` is the id of the character whose link is being edited, or null when adding one.

A Settings panel lives only while its section is shown: when the user switches to another section or closes Settings, the page is closed and its work stops. A `packs.install` already sent still finishes (or changes nothing). Keep work short, or save progress with `storage.set` so it can pick up again.

## The message API

The page sends calls to its parent window, and each one is answered once:

```js
// call:   { koldron: 1, id, call, args }
// answer: { koldron: 1, id, ok: true, result } or { koldron: 1, id, ok: false, error }
let next = 0;
const waiting = new Map();
addEventListener("message", (event) => {
  if (event.source !== parent || event.data?.koldron !== 1) return;
  const { id, ok, result, error } = event.data;
  waiting.get(id)?.[ok ? 0 : 1](ok ? result : new Error(error));
  waiting.delete(id);
});
function koldron(call, ...args) {
  return new Promise((resolve, reject) => {
    const id = ++next;
    waiting.set(id, [resolve, reject]);
    parent.postMessage({ koldron: 1, id, call, args }, "*");
  });
}
```

| Call | Arguments | Result |
|---|---|---|
| `packs.list` | | Every pack: `{ id, name, version, license, entries, enabled, installed, errors }`. `installed` is false for a pack bundled with the app. |
| `packs.install` | `{ manifest, files, images?, license? }` | The installed pack, as in `packs.list`. |
| `packs.remove` | pack id | `null` |
| `net.fetch` | `https://` URL | The body as an `ArrayBuffer` (at most 200 MB). A GET; an error status is an error. |
| `files.pickFile` | optional list of extensions, e.g. `["zip"]` | The file the user picked, as an `ArrayBuffer`, or `null` if they cancelled (at most 200 MB). |
| `files.pickFolder` | | `[{ path, text }]` for every file in the picked folder and its subfolders (`text` is null for a file that isn't UTF-8), or `null` if cancelled. |
| `characters.list` | campaign id | The campaign's characters linked to this addon. |
| `characters.save` | campaign id, character | The character as saved. Empty `id`: a new character, linked to this addon. Otherwise it must be one linked to this addon. The app keeps the player and party that were set in the app; everything else is the addon's. A linked character's name, classes, stats and death saves can't be changed in the app, while the addon is installed; without it the character is an ordinary one, and putting the addon back links it again. |
| `characters.setPicture` | campaign id, character id, image file as `ArrayBuffer` | The character as saved. Only for a character linked to this addon. |
| `storage.get` | | What `storage.set` saved last, or `null`. |
| `storage.set` | any JSON value (at most 10 MB) | `null` |
| `ui.setHeight` | pixels | `null`. The frame's height, at most 4000. |
| `ui.theme` | | `"light"` or `"dark"`, the app's current theme. |
| `ui.close` | | `null`. Closes the dialog the page is in. |
| `ui.notify` | message | `null`. Shows the message (at most 200 characters) as the app's notification, after the addon's name. |

A character is `{ id, name, player, classes: [{ name, level }], maxHp, currentHp, tempHp, ac, initiative, passivePerception, passiveInsight, passiveInvestigation, deathSaves: { successes, failures } | null, onHome, linked }`. Stats are whole numbers or null.

The picked file or folder's path is never handed over.

### packs.install

`manifest` is the pack's `pack.json`. `files` holds each data file the manifest names, by that path, as JSON. `images` is its `images.json` and `license` its license text (written as `LICENSE.md`). The pack is validated completely, exactly as one loaded from disk. An invalid pack is refused with its errors, and the pack installed before stays.

The pack is installed as `packs/<id>/` in the data folder, replacing the one installed with that id. An installed pack with the id of a bundled pack (e.g. `srd`) is used instead of it. Removing it brings the bundled one back. A bundled pack can't be removed. The app's search and Library pick up the change at once.
