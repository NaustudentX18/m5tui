# m5Tui — Community Theme Market

> A network-accessible catalog of community-uploaded themes. Browse,
> preview, install, and publish — all from the device, keyboard-first,
> 240×135-first.

---

## 1. The metaphor

A **theme market** is a tiny storefront for the visual identity of m5Tui. Themes are pure data (YAML + 3 small WAVs), so they're safe to share, easy to preview, and trivial to install.

The market has three roles:

- **Browse** — see what other m5Tui users have made. Sort by name, author, popularity, install count, recency.
- **Preview** — render a theme's palette and glyphs *on your current data* (the cockpit with your real profiles, your real session state) before you install. No surprises.
- **Publish** — share your own theme. One key, a one-line description, a handle. Done.

The market is **browse-first, install-second**. The whole thing is designed to be done in 30 seconds with the keyboard, in the dark, on a 240×135 screen.

---

## 2. The flow

`;m` opens the market. The layout (40×16):

```
+--------------------------------------------+
| MARKET  /browse  catalog: 248  net: ok     |
+--------------------------------------------+
| > Coldwire+  forest        ░▒▓ ★ 4.9  12k  |
|   Phosphor   adam         ░░▒ ★ 4.6   3k  |
|   Lacuna     nullsec      ▒▒▒ ★ 4.8   1k  |
|   Magline    leah         ▓▓▒ ★ 4.7   8k  |
|   Sunsetwave m            ▒▓▓ ★ 4.5  500  |
|   3am-coding you          ░▓▒ ★ 4.9  900  |
+--------------------------------------------+
| [Coldwire+]  cold + wire  cyan on midnight|
+--------------------------------------------+
```

- Top: status (catalog count, network).
- Middle: the catalog list. Each row shows the theme name, author, a 3-cell palette swatch, star rating, install count.
- Bottom: the highlighted theme's description.

`enter` opens the **preview** screen. `;i` installs. `;p` publishes the current theme.

---

## 3. The backend

The market is a tiny static setup. No servers to run, no databases to migrate.

| Piece | Host | What it stores |
|---|---|---|
| **Catalog** | GitHub Pages (or `aiserver-1`) | `catalog.json` — list of {name, author, version, sha256, size, rating, installs, description, preview_swatch} |
| **Theme assets** | Backblaze B2 / Cloudflare R2 / S3 | One folder per theme: `theme.yaml`, `boot.wav`, `click.wav`, `arp.wav` |
| **Ratings** | A small D1 / Turso / SQLite on a free tier | `(theme_id, stars)` aggregated client-side; submitted via signed POST |
| **Telemetry** | Same as ratings | `(theme_id, install_count)` — opt-in via the binary's `market.enable_telemetry` flag |

**Catalog schema:**

```json
{
  "version": 1,
  "generated_at": "2026-06-15T22:00:00Z",
  "themes": [
    {
      "id": "coldwire+",
      "name": "Coldwire+",
      "author": "forest",
      "version": "1.2.0",
      "sha256": "ab12...",
      "size_bytes": 8192,
      "stars": 4.9,
      "installs": 12345,
      "description": "Coldwire with extra synthwave and a glitch on every keystroke.",
      "preview_swatch": ["#0a1428", "#00d8ff", "#ff00aa"],
      "tested_omp_versions": ["15.13.3"],
      "tags": ["cyberpunk", "synthwave", "cyan"]
    }
  ]
}
```

The binary downloads the catalog, diffs against the local cache, and re-renders the list. Themes are downloaded on demand when previewed or installed.

---

## 4. The preview

A theme preview renders the **cockpit with the new theme** using the current data. This is the most important UX moment in the market — the user has to *see* the theme in context.

```
+--------------------------------------------+
| PREVIEW  Coldwire+    [install]  [share]   |
+--------------------------------------------+
| aiserver-1  :: OK  :: 73% ▁▃▆             |
+--------------------------------------------+
| AGENTS                | SESSION            |
| > aiserver-1/omp      | task 14/22  in_flt |
|   aiserver-1/bridge   | model MiniMax-M3  |
+--------------------------------------------+
| palette:  ░ bg  ▒ fg  ▓ accent  █ prompt   |
+--------------------------------------------+
| > _                                                |
+--------------------------------------------+
```

- The top half is the **live cockpit** with the preview theme applied.
- The bottom half shows the **palette swatches** with the role labels.
- `;i` installs. `;b` goes back. `tab` toggles between "compact" and "cozy" density to see how the theme scales.

The preview is **non-destructive** — it doesn't touch the active theme. `;b` returns to the active theme.

---

## 5. Publishing

`;mp` (or `;m p`) opens the publish flow:

```
+--------------------------------------------+
| PUBLISH THEME                              |
+--------------------------------------------+
| name:        Coldwire+____________________  |
| description: Coldwire with extra synthwave_  |
| handle:      forest______________________  |
| tags:        cyberpunk, synthwave__________  |
+--------------------------------------------+
| [palette preview]  [glyph preview]          |
+--------------------------------------------+
| [publish]  [cancel]                         |
+--------------------------------------------+
```

`;pub` finalises. The binary:

1. Validates the theme (schema check).
2. Computes `sha256` of the YAML.
3. POSTs the catalog entry to the ratings backend (with the handle as the author).
4. Uploads the YAML + WAVs to B2/R2/S3.
5. Shows a toast: "published as Coldwire+ v1.2.0".

**Auth:** publishing requires a handle. Handles are first-come-first-served; the binary stores the handle in `/m5tui/config.yaml` and signs the upload with a device-local key. No GitHub OAuth in v1 — friction matters more than provenance for a personal-device market.

If a handle is already taken, the binary appends `-<2-char-hash>` and asks the user to confirm.

---

## 6. Offline behaviour

- The **catalog** is cached at `/m5tui/market/catalog.json`. Picker works fully offline; the user just sees the last-known list.
- **Installed themes** live in `/m5tui/themes/` and work regardless of network state.
- **Preview** requires the theme YAML, which is cached on first preview.
- **Publish** queues the upload in `/m5tui/market/outbox/` and posts when the mesh returns.

When the network is down, the top bar shows `market: offline` so the user knows.

---

## 7. Safety

Themes are pure data. There is **no embedded code**. The loader:

- Validates the schema (`m5tui-themes::schema`).
- Caps file size at 16 KB YAML and 4 KB per WAV.
- Refuses to follow any symbolic links in the WAV paths.
- Sandboxes the colour picker preview to a single frame; the user must explicitly `;i` install to make it live.
- Keeps the previous theme active until `;i` is confirmed.

If a YAML is malicious (e.g. tries to overflow the schema with a billion palette entries), the loader truncates at the schema-defined max and rejects with a friendly error.

---

## 8. The market backend — the boring ops part

### 8.1 Hosting (proposed)

- **Catalog:** `https://m5tui.community.market/catalog.json`, served by GitHub Pages on `NaustudentX18/m5tui-market` (a tiny repo, just JSON + a static `index.html` with a browser preview).
- **Assets:** Backblaze B2 bucket `m5tui-themes`, public-read, with a Cloudflare CDN in front. Free tier covers 10 GB.
- **Ratings / telemetry:** Cloudflare D1 (free SQLite, 5 GB / 5M rows). Endpoint: `https://m5tui.community.market/api/rate`.

### 8.2 Moderation

In v1: **no moderation**. Themes are public on publish, no approval queue, no takedown. If a malicious theme is published, the user can flag it (`;f` in the market) which queues a moderation request; the operator (you) reviews and can ban the handle or revert the theme.

In v1.1: add a **community vote** — a theme with > 5 downvotes in 24 h gets auto-hidden.

### 8.3 Versioning

Themes have a `version` field. The Market tracks the latest version per `id`; older versions stay available for a "rollback" action (`;rr` in the picker).

When a theme updates, the user gets a toast on next boot: "Coldwire updated 1.1 → 1.2. Update? (y/n)". One-key update, one-key skip.

---

## 9. Discovery without the market

The market is the primary discovery surface, but not the only one:

- The **README** in the repo links to the catalog URL and showcases ~6 hand-picked themes.
- The **GitHub Discussions** category `#themes` is the place for "show off your theme" threads.
- Themes can be **pasted as a gist** and installed via `;mi https://gist.github.com/.../raw/theme.yaml` (the URL is downloaded, validated, installed).

---

## 10. The market and the Book of Commands

The Market and the Book are **separate surfaces that share a publishing path**:

- A **theme** is published via `;mp` → Market catalog.
- A **spell** is published via `;b p` → Spell catalog (which is a parallel catalog, same backend, different content type).

A future v1.1 may merge them into a single "Community Library" with type filters. For v1, they're separate because the safety story is different (themes are visual only; spells can run commands).

---

**End of MARKET.md.**
