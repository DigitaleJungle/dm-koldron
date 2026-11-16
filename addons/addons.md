# Koldron addons

An addon is code that extends DM's Koldron. It runs in a sandboxed frame inside the app and talks to it through a small message API. It can't reach the app's window, your files or the network except through that API, and only as far as the permissions in its manifest allow.

Content packs are not addons (see `packs/packs.md`). An addon can produce packs, for example an importer.

## Installing

There's no install screen yet. Put the addon's folder in the app's data folder, `addons/<id>/`, and restart the app. The app creates the empty `addons` folder when it starts:

- Windows: `%APPDATA%\be.digitalejungle.koldron\addons\`
- macOS: `~/Library/Application Support/be.digitalejungle.koldron/addons/`

A development build uses `be.digitalejungle.koldron.dev` instead. Settings → About lists the addons, and an invalid one with its errors.

Removing the folder removes the addon. Data it saved (`addon-data/<id>/`, with `storage.*File` files in `files/`) and packs it installed (`packs/<id>/`) stay.

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
  "background": "ui/background.html",
  "library": { "label": "My tab", "entry": "ui/tab.html" }
}
```

| Field | |
|---|---|
| `api` | The addon API version it's written for. This app speaks `1`; an addon for another version isn't loaded. |
| `id` | Lowercase letters, digits and dashes; the same as the folder name. |
| `name`, `version`, `license` | Required, shown in Settings → About. |
| `permissions` | Optional. See below. Unknown permissions make the addon invalid. |
| `settings` | Optional. A page shown as a panel in a Settings section: `rules` (below the pack list) or `library` (a Library section, shown only when an addon adds a panel to it). The addon's `library` page is closed while Settings is open and loads again when it closes. |
| `characters` | Optional. An item labelled `label` (at most 40 characters) in the Characters screen's **Add character** menu, opening the page `entry` in a dialog. Add character is a menu only when an addon adds an item. The same page opens from **Edit link** on the form of a character linked to the addon. `owns`: the fields of the characters it adds that the addon keeps up to date (see below). |
| `background` | Optional. A page that runs hidden for as long as the app is open. |
| `library` | Optional. A tab labelled `label` (at most 40 characters) in the app-wide Library, after Catalog, showing the page `entry` across the whole pane (not inside a campaign's Library). The page scrolls itself; keep its bottom-left corner clear, where the Settings gear sits. The addon can hide and show the tab (`library.setVisible`); a hidden tab isn't in the tab bar, and an address to it shows the Rules tab. |

### Permissions

| Permission | Allows |
|---|---|
| `packs:read` | `packs.list` |
| `packs:write` | `packs.install`, `packs.remove` |
| `folders:read` | `folders.list`, `folders.add`, `folders.remove`, `folders.scan`, `folders.read`, `folders.reveal` |
| `characters:write` | `characters.list`, `characters.save`, `characters.setPicture` |
| `net:<host>` | `net.fetch` to `https://<host>/…`, one permission per host name (no port, no wildcards). Only the address the addon asks for is checked; redirects the host sends are followed, wherever they lead (GitHub release downloads need them). |

Placing the folder is the consent: an addon gets exactly what it declares, and every other call is refused.

## The page

Pages are served from `koldron-addon://localhost/<id>/…` (on Windows `http://koldron-addon.localhost/<id>/…`). They run with an opaque origin, so there's no cookies, `localStorage` or IndexedDB: use `storage.*` instead. Their Content Security Policy allows scripts, styles, images, fonts, workers and `fetch` from the addon's own files only (plus `data:`/`blob:` images and `blob:` workers). There's no other network access: use `net.fetch`.

The page's address ends in `?theme=light` or `?theme=dark`, the app's theme when the page opened. Read it in a plain `<script src>` in the `<head>` (not a module, which runs after the first paint) to show the right theme from the start; `ui.theme` gives the same answer later.

Forms work as usual: listen for `submit` and call `preventDefault()`. A form can't be sent anywhere (the policy has `form-action 'none'`).

Run heavy work (unzipping, converting) in a Web Worker so the app's window stays responsive. Start it as a classic worker from a `blob:` URL (fetch the script and wrap it in a `Blob`): a worker from the addon's own URL, or a module worker (`{ type: "module" }`), is refused in the sandboxed frame.

### Context

The app tells a page where it is with a message (not a call, so there's no answer): `{ koldron: 1, event: "context", context }`. It's sent once the page has loaded and again whenever the context changes.

- Background page: `{ campaign, live, screen }`. `campaign` is the open campaign's id, or null. `live` is true while a running encounter or a running session is shown. `screen` is the path of the screen shown, so a new context arrives every time a screen loads.
- `characters` page: `{ campaign, character }`. `character` is the id of the character whose link is being edited, or null when adding one.
- `library` page: `{ state }`, the state the page last moved to with `ui.navigate` (a string), or null. It's in the address (`?tab=<id>&state=…`), so Back and Forward send it again as a new context; the page stays loaded and shows that state.

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
| `files.saveFile` | suggested file name, the file as an `ArrayBuffer` | `true`, or `false` if the user cancelled. The user picks where in the system dialog. |
| `folders.list` | | The folders the user gave the addon: `[{ id, name }]`. |
| `folders.add` | | The folder the user picked, `{ id, name }`, or `null` if they cancelled. The addon may read it from then on. |
| `folders.remove` | folder id | `null`. The addon can't read it any more; its files are never touched. |
| `folders.scan` | folder id | `[{ path, size, modified }]` for every file in the folder and its subfolders (`modified` in ms). |
| `folders.read` | folder id, path, offset, length | Those bytes of the file as an `ArrayBuffer` (fewer at its end; at most 16 MB per call). |
| `folders.reveal` | folder id, path | `null`. Shows the file in the system file manager (Explorer, Finder), selected. An error on other systems. |
| `characters.list` | campaign id | The campaign's characters linked to this addon. |
| `characters.save` | campaign id, character | The character as saved. Empty `id`: a new character, linked to this addon. Otherwise it must be one linked to this addon. When creating, the addon sets everything. After that it changes only the fields its manifest `owns`; everything else is the app's. |
| `characters.setPicture` | campaign id, character id, image file as `ArrayBuffer` | The character as saved. Only for a character linked to this addon. |
| `storage.get` | | What `storage.set` saved last, or `null`. |
| `storage.set` | any JSON value (at most 10 MB) | `null` |
| `storage.putFile` | file name, the file as an `ArrayBuffer` (at most 50 MB) | `null`. Keeps it as the addon's own file, replacing one with that name. A name is 1–200 ASCII letters, digits, `.`, `_` and `-`, not starting or ending with `.`; not ending in `.tmp`, and not a Windows device name (`con`, `nul.txt`…). |
| `storage.getFile` | file name | The file as an `ArrayBuffer`, or `null` when there's no such file. |
| `storage.deleteFile` | file name | `null`, also when there's no such file. |
| `storage.listFiles` | | `[{ name, size, modified }]`, the addon's files by name (`modified` in ms). |
| `library.setVisible` | `true` or `false` | `null`. Shows or hides the addon's Library tab, at once and after a restart. Only for an addon with a `library` tab. |
| `ui.setHeight` | pixels | `null`. The frame's height, at most 4000. |
| `ui.theme` | | `"light"` or `"dark"`, the app's current theme. |
| `ui.close` | | `null`. Closes the dialog the page is in. |
| `ui.notify` | message | `null`. Shows the message (at most 200 characters) as the app's notification, after the addon's name. |
| `ui.navigate` | state (a string of at most 2000 characters, or `null`), optional `replace` | `null`. From the `library` page only: moves the app to that state of the tab as a step Back returns from, or in place of the current one with `replace: true`. The page then gets the new context. |

### Owned fields

`characters.owns` lists the fields an addon keeps up to date on the characters it added: any of `name`, `player`, `classes`, `maxHp`, `currentHp`, `tempHp`, `ac`, `initiative`, `passivePerception`, `passiveInsight`, `passiveInvestigation`, `deathSaves`. While the addon is installed, those can't be changed in the app (its form shows them locked; in a running encounter, owned HP, temp HP and death saves follow the character and aren't written back at the end). The others are the app's. Without the addon, the character is an ordinary one; putting the addon back picks it up again. The picture is never owned: both the app and `characters.setPicture` can change it.

A character is `{ id, name, player, classes: [{ name, level }], maxHp, currentHp, tempHp, ac, initiative, passivePerception, passiveInsight, passiveInvestigation, deathSaves: { successes, failures } | null, onHome, linked }`. Stats are whole numbers or null.

The picked file or folder's path is never handed over. A folder from `folders.add` is known to the addon only by its id and name.

### packs.install

`manifest` is the pack's `pack.json`. `files` holds each data file the manifest names, by that path, as JSON. `images` is its `images.json` and `license` its license text (written as `LICENSE.md`). The pack is validated completely, exactly as one loaded from disk. An invalid pack is refused with its errors, and the pack installed before stays.

The pack is installed as `packs/<id>/` in the data folder, replacing the one installed with that id. An installed pack with the id of a bundled pack (e.g. `srd`) is used instead of it. Removing it brings the bundled one back. A bundled pack can't be removed. The app's search and Library pick up the change at once.
