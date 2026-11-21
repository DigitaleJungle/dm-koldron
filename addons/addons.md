# Koldron addons

An addon is code that extends DM's Koldron. It runs in a sandboxed frame inside the app and talks to it through a small message API. It can't reach the app's window, your files or the network except through that API, and only as far as the permissions in its manifest allow.

Content packs are not addons (see `packs/packs.md`). An addon can produce packs, for example an importer.

## Installing

In **Settings → Addons**, **Install addon…** and pick the addon's `.zip`: its files at the top of the zip, or the addon's folder zipped (one folder with `addon.json` in it). The app checks it, shows the permissions it asks for, and installs it when you click **Save**; an addon with the same id is replaced, its saved data kept. It loads after a restart (**Restart now**).

Or put the addon's folder in the app's data folder, `addons/<id>/`, and restart the app. The app creates the empty `addons` folder when it starts:

- Windows: `%APPDATA%\be.digitalejungle.koldron\addons\`
- macOS: `~/Library/Application Support/be.digitalejungle.koldron/addons/`

A development build uses `be.digitalejungle.koldron.dev` instead. Settings → Addons lists the addons, and an invalid one with its errors.

Settings → Addons can also disable, enable and uninstall an addon (on **Save**, taking effect after a restart). Uninstalling, or removing the folder by hand, removes the addon. Data it saved (`addon-data/<id>/`, with `storage.*File` files in `files/`) and packs it installed (`packs/<id>/`) stay, unless you tick **Also delete its saved data and secrets** when uninstalling: then its data folder and the secrets it stored with `secrets.set` go too (packs always stay).

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
  "library": { "label": "My tab", "entry": "ui/tab.html" },
  "home": { "label": "My widget", "entry": "ui/home.html" },
  "campaign": { "label": "My screen", "icon": "map-2", "entry": "ui/screen.html", "sidebar": true, "visible": true }
}
```

| Field | |
|---|---|
| `api` | The addon API version it's written for. This app speaks `1`; an addon for another version isn't loaded. |
| `id` | Lowercase letters, digits and dashes; the same as the folder name. |
| `name`, `version`, `license` | Required, shown in Settings → Addons. |
| `permissions` | Optional. See below. Unknown permissions make the addon invalid. |
| `settings` | Optional. A page shown as a panel in a Settings section, `{ section, entry }`, or a list of them, at most one per section: `rules` (below the pack list), `library` (a Library section) or `integrations` (an Integrations section, for connections to other services). Library and Integrations show only when an addon adds a panel to them. In Library every addon's panel is shown one under another; Integrations shows each panel as a tab named by its optional `label` (at most 40 characters; the addon's name without one), so the panel needs no heading of its own. The addon's `library` page is closed while Settings is open and loads again when it closes. |
| `characters` | Optional. An item labelled `label` (at most 40 characters) in the Characters screen's **Add character** menu, opening the page `entry` in a dialog. Add character is a menu only when an addon adds an item. The same page opens from **Edit link** on the form of a character linked to the addon. `owns`: the fields of the characters it adds that the addon keeps up to date (see below). |
| `background` | Optional. A page that runs hidden for as long as the app is open. |
| `start` | Optional. A page on the Start screen, in a column beside the campaigns (under them in a narrow window). The page sets its own height with `ui.setHeight`; at 0 it's hidden, and the column takes no room until one of its pages is taller than 0. Call `ui.setHeight` once the page has drawn what it shows: until then the frame is 0 tall and out of sight, and the browser doesn't run resize observers in it. From there, `ui.openLibrary` opens the addon's Library tab and `ui.openCampaignScreen` its campaign screen. |
| `preview` | Optional. A page shown in the search screen's preview pane, under the result's name and the app's Open button, for the addon's own search results (`search.setEntries`); without it, the pane shows only those two. Its context says which entry (see Context); the page sets its own height with `ui.setHeight`, the page sets its own height with `ui.setHeight`, and can open the addon's Library tab with `ui.openLibrary` or its campaign screen with `ui.openCampaignScreen`. | | Optional. A tab labelled `label` (at most 40 characters) in the app-wide Library, after Catalog unless reordered in Settings → UI, showing the page `entry` across the whole pane (not inside a campaign's Library). The page scrolls itself; keep its bottom-left corner clear, where the Settings gear sits. With `"sidebar": true` the page has a side column: the Library's header then shows a button to hide or show it (remembered per addon on this device), and the page hears which in its context. The addon can hide and show the tab (`library.setVisible`); a hidden tab isn't in the tab bar, and an address to it shows the first tab. |
| `campaign` | Optional. A screen in every campaign's sidebar, labelled `label` (at most 40 characters) with the icon `icon`: a [Tabler icon](https://tabler.io/icons) name, e.g. `map-2`, or an `.svg` file in the addon folder (at most 16 KB, e.g. `ui/mark.svg`), drawn as a mask in the text colour like the app's icons, so only its shape counts (anything else in the file is ignored, and nothing in it runs), after the app's own screens until moved (the sidebar's grip moves it, like the others). Its page `entry` fills the screen below the campaign header and scrolls itself; it hears the campaign in its context. With `"sidebar": true` the page has a side column: the campaign header then shows a button to hide or show it (remembered per addon on this device), and the page hears which in its context. The page is closed while Settings is open and loads again when it closes. The addon can hide and show it per campaign (`campaign.setVisible`); `"visible": false` hides it in every campaign until the addon shows it there. Hidden, or without the addon, the item is gone and its address shows the campaign's Home. |
| `home` | Optional. A widget on every campaign's Home, showing the page `entry`. `label` (at most 40 characters) is its name for screen readers and in **Add widget**. It moves, resizes and comes off Home like the app's own widgets (one column wide at first). The page sets its own height with `ui.setHeight`, hears which campaign and its width in its context, and can open the addon's Library tab with `ui.openLibrary` or its campaign screen with `ui.openCampaignScreen`. |

### Permissions

| Permission | Allows |
|---|---|
| `packs:read` | `packs.list` |
| `packs:write` | `packs.install`, `packs.remove`, `packs.putImages`, `packs.setProgress` |
| `folders:read` | `folders.list`, `folders.add`, `folders.remove`, `folders.scan`, `folders.read`, `folders.hash`, `folders.reveal` |
| `files:read` | `files.open`, `files.read` |
| `folders:write` | `folders.download` (with `folders:read`, and `net:<host>` for the address) |
| `secrets` | `secrets.get`, `secrets.set`, `secrets.delete` |
| `search` | `search.setEntries` |
| `characters:write` | `characters.list`, `characters.save`, `characters.setPicture` |
| `campaigns:read` | `campaigns.list` |
| `net:<host>` | `net.fetch` to `https://<host>/…`, one permission per host name (no port, no wildcards). Only the address the addon asks for is checked; redirects the host sends are followed, wherever they lead (GitHub release downloads need them). |
| `links:<host>` | `ui.openExternal` to `https://<host>/…`, one permission per host name, as `net:<host>`. |

Placing the folder is the consent: an addon gets exactly what it declares, and every other call is refused.

## The page

Pages are served from `koldron-addon://localhost/<id>/…` (on Windows `http://koldron-addon.localhost/<id>/…`). They run with an opaque origin, so there's no cookies, `localStorage` or IndexedDB: use `storage.*` instead. Their Content Security Policy allows scripts, styles, images, fonts, workers and `fetch` from the addon's own files only (plus `data:`/`blob:` images and `blob:` workers). There's no other network access: use `net.fetch`.

The page's address ends in `?theme=light` or `?theme=dark`, the app's theme when the page opened. Read it in a plain `<script src>` in the `<head>` (not a module, which runs after the first paint) to show the right theme from the start; `ui.theme` gives the same answer later. It also ends in `&motion=reduce` or `&motion=full`, the app's Settings → Animations (which can differ from the system's reduce-motion): skip animations on `reduce`.

Forms work as usual: listen for `submit` and call `preventDefault()`. A form can't be sent anywhere (the policy has `form-action 'none'`).

Run heavy work (unzipping, converting) in a Web Worker so the app's window stays responsive. Start it as a classic worker from a `blob:` URL (fetch the script and wrap it in a `Blob`): a worker from the addon's own URL, or a module worker (`{ type: "module" }`), is refused in the sandboxed frame.

### Context

The app tells a page where it is with a message (not a call, so there's no answer): `{ koldron: 1, event: "context", context }`. It's sent once the page has loaded and again whenever the context changes.

- Background page: `{ campaign, live, screen }`. `campaign` is the open campaign's id, or null. `live` is true while a running encounter or a running session is shown. `screen` is the path of the screen shown, so a new context arrives every time a screen loads.
- `characters` page: `{ campaign, character }`. `character` is the id of the character whose link is being edited, or null when adding one.
- `home` page: `{ campaign, width }`, the campaign whose Home it's on and the widget's width in columns (1 to 3), sent again when it's resized.
- `preview` page: `{ id, state }`, the entry shown: its `id` and `state` as the addon gave them to `search.setEntries`. Sent again when another of the addon's results is picked; the page stays loaded.
- `start` page: `{ tileHeight, tileTop }`, a campaign tile's height and its top measured from the top of the page's column, in pixels (null when there are no campaigns, or when the column is under them in a narrow window), sent again when they change, so the page can line up with the tiles.
- `campaign` page: `{ campaign, state }`, the campaign shown and the state the page last moved to with `ui.navigate`, as for a `library` page (`?state=` in the address); with `"sidebar": true` also `sidebarHidden`.
- `library` page: `{ state }`, the state the page last moved to with `ui.navigate` (a string), or null; with `"sidebar": true` also `sidebarHidden` (true or false), set by the header's button: hide or show the side column when it changes. It's in the address (`?tab=<id>&state=…`), so Back and Forward send it again as a new context; the page stays loaded and shows that state.

A Settings panel stays loaded while the Settings dialog is open (hidden while another section is shown) and closes with it; its work then stops. A `packs.install` already sent still finishes (or changes nothing). Keep work short, or save progress with `storage.set` so it can pick up again.

### Saving settings

The Settings dialog has one **Save** for all its sections; **Close** (or the ✕, Escape) with changes not saved asks first (Save, Discard or Keep editing). A panel joins in like this:

- When something on it changes, it calls `ui.setUnsaved(true)` and keeps the change to itself (nothing is saved yet); `ui.setUnsaved(false)` when it's back as saved.
- The app then sends `{ koldron: 1, event: "settings", action, id }`: `action` is `"save"` (keep the changes; start what they lead to, or leave that to another page) or `"discard"` (go back to what's saved, and undo what was already done for the changes, such as a folder given with `folders.add`). It is sent only to a panel that said it has unsaved changes.
- The panel answers with `ui.settingsDone(id, null)` when it's done, or `ui.settingsDone(id, "what went wrong")`; the dialog shows that, after the addon's name, and stays open. A panel that doesn't answer within 2 minutes (10 seconds for a discard) counts as failed.

Actions with a button of their own (scan, delete, export…) stay actions: they happen when pressed. A panel that never calls `ui.setUnsaved` keeps saving its changes itself, at once.

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
| `packs.remove` | pack id | `null`. Its kept images (`images/<pack>/`) go too, unless a bundled pack with that id comes back. |
| `packs.putImages` | pack id, a list of `{ path, bytes }` (`bytes` an `ArrayBuffer`) | `null`. Keeps the images for a pack this addon installed with `packs.install`, where the app keeps a pack's images. An entry whose `image` is one of those paths (with or without a leading `/`) shows it, without **Enable images**, as long as the pack has no `images.json`. `path`: relative, plain ASCII, no `..`, ending in `.png`, `.jpg`, `.jpeg` or `.webp` (lowercase). At most 100 images and 100 MB per call, 20 MB each; an image with the same path is replaced. Installing the pack again keeps them. |
| `packs.setProgress` | pack id, `{ name, fraction, text }` or `null` | `null`. Shows the addon's work on a pack in its Settings → Rules row: a bar (`fraction` 0–1, or `null` for a moving one) and `text`, in place of its version, entries and license. For a pack not installed yet (its first import), a row with `name` shows until it's installed or cleared. Only for a pack this addon installed, or one that isn't installed. `null` clears it; it also clears when the page that set it closes. Name and text: at most 100 characters. |
| `net.fetch` | `https://` URL, optional `{ method, headers }` | The body as an `ArrayBuffer` (at most 200 MB). `method` is `"GET"` (the default) or `"POST"` (sent without a body); `headers` is an object of names and values (at most 20; not `Host`, `Cookie` or the connection's own). An error status is an error that names it (e.g. `… http status: 401`); messages leave out the address's query. |
| `files.pickFile` | optional list of extensions, e.g. `["zip"]` | The file the user picked, as an `ArrayBuffer`, or `null` if they cancelled (at most 200 MB). |
| `files.open` | optional list of extensions, e.g. `["gz"]` | The files the user picked (several at once): `[{ id, name, size }]`, or `null` if they cancelled. The addon reads them with `files.read` until the app closes. |
| `files.read` | file id from `files.open`, offset, length | Those bytes of the file as an `ArrayBuffer` (fewer at its end; at most 16 MB per call). |
| `files.pickFolder` | | `[{ path, text }]` for every file in the picked folder and its subfolders (`text` is null for a file that isn't UTF-8), or `null` if cancelled. |
| `files.saveFile` | suggested file name, the file as an `ArrayBuffer` | `true`, or `false` if the user cancelled. The user picks where in the system dialog. |
| `folders.list` | | The folders the user gave the addon: `[{ id, name }]`. |
| `folders.add` | | The folder the user picked, `{ id, name }`, or `null` if they cancelled. The addon may read it from then on. |
| `folders.remove` | folder id | `null`. The addon can't read it any more; its files are never touched. |
| `folders.scan` | folder id | `[{ path, size, modified }]` for every file in the folder and its subfolders (`modified` in ms). |
| `folders.read` | folder id, path, offset, length | Those bytes of the file as an `ArrayBuffer` (fewer at its end; at most 16 MB per call). |
| `folders.hash` | folder id, path | The file's MD5 checksum, lowercase hex. |
| `folders.download` | folder id, path, `https://` URL | The downloaded file's MD5. Downloads the URL (a `net:<host>`; redirects followed) to that path in the folder (at most 8 GB, 30 minutes), making its folders. It goes to `<path>.download` first and replaces the path only once complete, so a broken download never replaces a good file. A path ending in `.download` is refused, and so is a download of a path another download is still writing. |
| `folders.reveal` | folder id, path | `null`. Shows the file in the system file manager (Explorer, Finder), selected. An error on other systems. |
| `campaigns.list` | | The active campaigns, most recently opened first: `[{ id, name }]`. |
| `characters.list` | campaign id | The campaign's characters linked to this addon. |
| `characters.save` | campaign id, character | The character as saved. Empty `id`: a new character, linked to this addon. Otherwise it must be one linked to this addon. When creating, the addon sets everything. After that it changes only the fields its manifest `owns`; everything else is the app's. |
| `characters.setPicture` | campaign id, character id, image file as `ArrayBuffer` | The character as saved. Only for a character linked to this addon. |
| `search.setEntries` | a list of `{ id, name, tags?, summary?, text?, state?, campaign? }` | `null`. Replaces what the addon makes findable in global search (at most 50,000 entries and 50 MB of text together; `id` 1–200 characters, `name` at most 500, at most 50 tags of 100, `summary` 2,000, `text` 20,000, `state` 2,000). The app keeps them, so search finds them from the start, inside campaigns too. An entry is found by its name, then its tags, then its summary and text; it shows the addon's name and its tags, its preview the addon's `preview` page, and opening it opens the addon's Library tab at `state` (as `ui.navigate`). An entry with a `campaign` (a campaign's id) belongs to that campaign: it's found like that campaign's characters (inside another campaign only with **Search everything**), opens in the addon's `campaign` screen there at `state` while that screen shows in that campaign, and is left out while the campaign isn't active (deleted or archived). Send them again whenever they change. |
| `secrets.get` | name | The secret saved under that name, or `null`. Secrets are kept in the system keychain, apart for each addon, never in a file. A name is 1–64 letters, digits, `-`, `_` or `.`. |
| `secrets.set` | name, the secret (a string of 1–2048 bytes) | `null` |
| `secrets.delete` | name | `null`, also when there's none. |
| `storage.get` | | What `storage.set` saved last, or `null`. |
| `storage.set` | any JSON value (at most 10 MB) | `null` |
| `storage.putFile` | file name, the file as an `ArrayBuffer` (at most 50 MB) | `null`. Keeps it as the addon's own file, replacing one with that name. A name is 1–200 ASCII letters, digits, `.`, `_` and `-`, not starting or ending with `.`; not ending in `.tmp`, and not a Windows device name (`con`, `nul.txt`…). |
| `storage.getFile` | file name | The file as an `ArrayBuffer`, or `null` when there's no such file. |
| `storage.deleteFile` | file name | `null`, also when there's no such file. |
| `storage.listFiles` | | `[{ name, size, modified }]`, the addon's files by name (`modified` in ms). |
| `campaign.setVisible` | campaign id, `true` or `false` | `null`. Shows or hides the addon's campaign screen in that campaign, at once and after a restart. Only for an addon with a `campaign` screen. |
| `library.setVisible` | `true` or `false` | `null`. Shows or hides the addon's Library tab, at once and after a restart. Only for an addon with a `library` tab. |
| `ui.setHeight` | pixels | `null`. The frame's height, at most 4000. |
| `ui.theme` | | `"light"` or `"dark"`, the app's current theme. |
| `ui.close` | | `null`. Closes the dialog the page is in. |
| `ui.notify` | message | `null`. Shows the message (at most 200 characters) as the app's notification, after the addon's name. |
| `ui.navigate` | state (a string of at most 2000 characters, or `null`), optional `replace` | `null`. From the `library` or `campaign` page only: moves the app to that state of the tab as a step Back returns from, or in place of the current one with `replace: true`. The page then gets the new context. |
| `ui.openLibrary` | state (a string of at most 2000 characters, or `null`) | `null`. From the `start`, `preview` and `home` pages only, of an addon with a shown `library` tab: opens that tab at the state (as `ui.navigate` moves it), as a step Back returns from. |
| `ui.openCampaignScreen` | campaign id, state (a string of at most 2000 characters, or `null`) | `null`. From the `start`, `preview` and `home` pages only, of an addon with a `campaign` screen shown in that campaign (`campaign.setVisible`): opens that screen in the campaign at the state (as `ui.navigate` moves it), as a step Back returns from. |
| `ui.openExternal` | `https://` address (at most 2000 characters, URL characters only) | `null`. Opens it in the system's browser; its host needs a `links:<host>` permission. Not from the `background` page. |
| `ui.setSidebarButton` | `true` or `false` | `null`. From a `library` or `campaign` page with `"sidebar": true` only: shows or hides the header's sidebar button, for while the page has no side column to show (the button's place stays, so the tabs don't move). Each time the tab opens, the button shows. |
| `ui.setUnsaved` | `true` or `false` | `null`. From a Settings panel only: it has (or no longer has) changes waiting for the dialog's Save (see Saving settings). |
| `ui.settingsDone` | the `settings` event's `id`, `null` or an error message | `null`. From a Settings panel only: it saved or discarded as asked, or couldn't. |

### Owned fields

`characters.owns` lists the fields an addon keeps up to date on the characters it added: any of `name`, `player`, `classes`, `maxHp`, `currentHp`, `tempHp`, `ac`, `initiative`, `passivePerception`, `passiveInsight`, `passiveInvestigation`, `deathSaves`. While the addon is installed, those can't be changed in the app (its form shows them locked; in a running encounter, owned HP, temp HP and death saves follow the character and aren't written back at the end). The others are the app's. Without the addon, the character is an ordinary one; putting the addon back picks it up again. The picture is never owned: both the app and `characters.setPicture` can change it.

A character is `{ id, name, player, classes: [{ name, level }], maxHp, currentHp, tempHp, ac, initiative, passivePerception, passiveInsight, passiveInvestigation, deathSaves: { successes, failures } | null, onHome, linked }`. Stats are whole numbers or null.

The picked file or folder's path is never handed over (`files.open` too: ids only, and only the addon that opened a file can read it). A folder from `folders.add` is known to the addon only by its id and name.

### packs.install

`manifest` is the pack's `pack.json`. `files` holds each data file the manifest names, by that path, as JSON. `images` is its `images.json` and `license` its license text (written as `LICENSE.md`). The pack is validated completely, exactly as one loaded from disk. An invalid pack is refused with its errors, and the pack installed before stays.

The pack is installed as `packs/<id>/` in the data folder, replacing the one installed with that id. An installed pack with the id of a bundled pack (e.g. `srd`) is used instead of it. Removing it brings the bundled one back. A bundled pack can't be removed. The app's search and Library pick up the change at once.
