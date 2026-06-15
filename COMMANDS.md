# m5Tui — Book of Commands

> The "how do I remember all this?" problem, solved. The Book of Commands
> is a first-class, on-device, discoverable library of named commands
> you can run with a single key or a single prefix. It's the difference
> between "I have to memorise 30 hotkeys" and "I press one key and the
> book lists everything I can do."

---

## 1. The metaphor

A **book of commands** is a list of *spells* the operator can cast. Each spell has:

- A **name** (one word, evocative, easy to remember)
- A **category** (the "school" of magic)
- A **single key** or **single prefix** binding (the incantation gesture)
- A **description** (one line — what it does)
- An **expanded command** (the actual magic — what runs)
- **Parameter prompts** (optional — what the wizard asks for)
- **Help text** (optional — the longer lore)
- **Undo** (optional — the counter-spell)

When you press `;b`, the book opens. You can:

- **Browse** — see every spell, grouped by school, with the binding next to it.
- **Search** — fuzzy match the name, description, or category.
- **Cast** — pick a spell, fill in any parameters, watch it run.
- **Learn** — `;b L` opens the spell author mode (data, not code — see §5).

The book is **data**, not code. Every spell is a YAML file. You can hand-edit them on the device, share them as files, or publish them to the Community Market.

---

## 2. The default book

m5Tui ships with ~30 default spells, organised by school. Bindings are chosen so the **first letter of the spell name** matches the binding (after `;`).

| School | Spell | Binding | Description |
|---|---|---|---|
| **agent** | `cast` | `;c` | Ask the OMP agent a one-shot question. |
|  | `summon` | `;su` | Summon a sub-agent for a focused task. |
|  | `forge` | `;f` | Run `advdeck-bridge plan` to forge a project from a rough idea. |
|  | `continue` | `;cc` | Continue the most recent OMP session. |
|  | `interrupt` | `;ci` | Send Ctrl-C to the running OMP session. |
| **harness** | `heal` | `;h` | Retry the most recent failed bridge job. |
|  | `banish` | `;b a` | Kill a stuck job (asks for the request id). |
|  | `export` | `;b e` | Export a project's agent pack to a host path. |
|  | `reweave` | `;b r` | Re-run the planner on a project's `idea.md`. |
| **memory** | `scry` | `;sc` | Scry the vault for a query (obsidian-memory). |
|  | `chart` | `;ch` | Chart a project's handoff (`agent-prompt.md`). |
|  | `log` | `;l` | Tail the OMP session log. |
| **voice** | `voice` | `;v` | Hold to record; release to push. |
|  | `play` | `;p` | Play the last voice memo. |
|  | `transcripts` | `;vt` | List the voice memos in `~/voice-inbox/`. |
| **shell** | `connect` | `;c c` | Connect to a profile. |
|  | `disconnect` | `;c d` | Disconnect from the current profile. |
|  | `reconnect` | `;c r` | Reconnect with keepalive. |
|  | `shell` | `;sh` | Drop into a raw PTY in the current profile. |
|  | `uptime` | `;su u` | Show aiserver uptime + load. |
|  | `disk` | `;su d` | Show aiserver disk space. |
| **theme** | `weave` | `;tw` | Weave (edit) a theme on the device. |
|  | `anoint` | `;ta` | Anoint (apply) a theme by name. |
|  | `paint` | `;tp` | Open the colour picker. |
| **market** | `market` | `;m` | Open the Community Theme Market. |
|  | `install` | `;m i` | Install a theme from the Market. |
|  | `publish` | `;m p` | Publish the current theme to the Market. |
| **system** | `wizard` | `;b w` | Re-run the first-boot wizard. |
|  | `doctor` | `;D` | Run `m5tui doctor`. |
|  | `about` | `;? ?` | Show the about screen. |
|  | `quit` | `;q q` | Quit m5Tui (asks for confirmation). |
|  | `reboot` | `;q r` | Reboot the Cardputer. |

**Note on binding collisions:** some spells share a first letter (`b` for banish, book-open, brightness). They use **two-key sequences** (`;b a`, `;b w`). The book list shows the full binding, so the user only needs to remember one mnemonic: the spell name. The binding is a hint, not a memorisation exercise.

---

## 3. Spell schema

```yaml
# book/cast.yaml
schema_version: 1
name: cast                       # the spell name
category: agent                  # school
description: "Ask the OMP agent a one-shot question"   # one line, shown in the book list
binding: ";c"                    # the incantation gesture
params:                          # optional, in order
  - name: prompt
    message: "ask?"
    multiline: true
    default: ""                  # optional, prefills the input
template: |                      # the expanded command
  ;ask {prompt}
help: |                          # optional, multi-line
  Cast opens the OMP prompt pre-filled with your question. Long-press
  ;c to open with a multi-line input for longer briefs.

  Examples:
    ;cast "what's on the bridge queue?"
    ;cast "summarise today's voice memos"
undo: null                       # optional, a command to run on ;u to reverse
```

### 3.1 Param schema

```yaml
- name: prompt                   # the variable name in the template
  message: "ask?"                # the prompt message
  multiline: false               # true for multi-line input
  default: ""                    # optional default value
  choices: []                    # optional, restricts to a list (renders as picker)
  secret: false                  # true for passwords / keys (typed value is masked)
```

### 3.2 Template syntax

The template is a string with `{name}` placeholders. Each placeholder is replaced by the user's input (after the prompt). The expanded command is sent to the active profile's SSH session (or to the local PTY if no profile is active).

Special placeholders:

- `{profile}` — the active profile name
- `{host}` — the active profile's hostname
- `{user}` — the active profile's user
- `{cwd}` — the active profile's working dir
- `{date}` — the current ISO date on the device
- `{time}` — the current ISO time on the device

### 3.3 Undo

If `undo:` is set, pressing `;u` within 30 s of the spell running will execute the undo command (after a confirmation). Useful for `forge` (undo = delete the project folder) and `anoint` (undo = restore the previous theme).

---

## 4. The book UI

`;b` opens the book. The layout (40×16):

```
+--------------------------------------------+
| BOOK OF COMMANDS       search: ___         |
+--------------------------------------------+
| AGENT                                       |
|  c  cast       Ask the OMP agent a question |
|  f  forge      Run advdeck-bridge plan      |
|  su summon     Summon a sub-agent           |
+--------------------------------------------+
| HARNESS                                     |
|  h  heal       Retry the last failed job    |
|  ba banish     Kill a stuck job             |
|  be export     Export a project's pack      |
+--------------------------------------------+
| > cast  ask?  ____________________________|
+--------------------------------------------+
```

- Top: the search box. Type to fuzzy-filter.
- Middle: the spell list, grouped by category. `↑/↓` to move, `enter` to cast.
- Bottom: the parameter prompt when a spell with params is being cast.

The bottom bar cycles:

1. `browse` — see all spells, pick one.
2. `param` — fill in the params.
3. `confirm` — show the expanded command, "run? (y/n)".
4. `cast` — execute.
5. `result` — show the result (toast + first 4 lines of output).

---

## 5. Authoring spells

`;b L` (or `;b A` on alternate bindings) opens **spell author mode**. This is the book equivalent of the theme editor:

```
+--------------------------------------------+
| NEW SPELL                                  |
+--------------------------------------------+
| name:        ____________________________  |
| category:    [agent] [harness] [memory] ... |
| description: ____________________________  |
| binding:     ;__                            |
| params:      [+] add param                  |
| template:    ____________________________  |
|              ____________________________  |
| help:        ____________________________  |
|              ____________________________  |
| undo:        ____________________________  |
+--------------------------------------------+
| [save]  [test]  [cancel]                    |
+--------------------------------------------+
```

`[+]` adds a param row. `[test]` expands the template with sample inputs and shows the result (no execution). `[save]` writes to `/m5tui/book/<name>.yaml`. `[cancel]` discards.

A user can also **export a spell** as a single YAML file (`;b e`) and **import** (`;b i`). A spell can be **published to the Community Market** (`;m p` from the book, or `;b p`).

---

## 6. The book and the profile

A profile can override the default book with a per-profile book. The resolution order:

1. `/m5tui/profiles/<profile>/book/*.yaml` (per-profile)
2. `/m5tui/book/*.yaml` (user)
3. The embedded default book (in the binary)

Per-profile spells shadow user spells with the same name. This lets you have a `cast` spell for `aiserver-1` that's different from the default `cast` (e.g. it uses a different default model).

---

## 7. Discovery in practice

The book solves the memorisation problem in three ways:

1. **The list is always one key away.** `;b` shows everything you can do. You don't have to remember — you browse.
2. **The bindings are derived from the names.** Once you learn that `c` = cast, you can guess that `f` = forge. Two-letter collisions (`;b a` for banish) are rare and shown in the list.
3. **The first-boot wizard walks you through the top 10.** On first boot, after the Wi-Fi/Tailscale steps, the wizard shows a 10-screen "tour" of the most-used spells. You can skip it, but it's the fastest way to internalise the verbs.

The book also **renders a one-line suggestion in the prompt** as you type. If you start typing `cas`, the prompt shows `;c  cast  Ask the OMP agent…` underneath. Press `;c` and the rest is filled in.

---

## 8. Bundled default book (full list)

| School | Spell | Binding | Template | Params |
|---|---|---|---|---|
| agent | `cast` | `;c` | `;ask {prompt}` | `prompt` (ml) |
| agent | `summon` | `;su` | `omp-subagent spawn --task {task}` | `task` |
| agent | `forge` | `;f` | `advdeck-bridge plan --project {project} --storage-root /advdeck` | `project` |
| agent | `continue` | `;cc` | `omp --continue --mode rpc-ui` | — |
| agent | `interrupt` | `;ci` | (sends Ctrl-C) | — |
| harness | `heal` | `;h` | `advdeck-bridge retry --last` | — |
| harness | `banish` | `;ba` | `advdeck-bridge cancel {request_id}` | `request_id` |
| harness | `export` | `;be` | `advdeck-bridge export --project {project} --out ./export` | `project` |
| harness | `reweave` | `;br` | `advdeck-bridge plan --project {project} --replan` | `project` |
| memory | `scry` | `;sc` | `obsidian-memory search "{query}"` | `query` |
| memory | `chart` | `;ch` | `cat ~/projects/{project}/agent-prompt.md` | `project` |
| memory | `log` | `;l` | `tail -n {lines} ~/logs/omp.log` | `lines` (default 50) |
| voice | `voice` | `;v` | (PTT) | — |
| voice | `play` | `;p` | `m5tui-voice play --last` | — |
| voice | `transcripts` | `;vt` | `ls -lat ~/voice-inbox/ \| head -20` | — |
| shell | `connect` | `;cc` (alt) | `m5tui connect {profile}` | `profile` (choices) |
| shell | `disconnect` | `;cd` | `m5tui disconnect` | — |
| shell | `reconnect` | `;cr` | `m5tui reconnect` | — |
| shell | `shell` | `;sh` | (drops into PTY) | — |
| shell | `uptime` | `;su u` | `uptime` | — |
| shell | `disk` | `;su d` | `df -h /` | — |
| theme | `weave` | `;tw` | (opens theme editor) | — |
| theme | `anoint` | `;ta` | `m5tui theme set {name}` | `name` (choices) |
| theme | `paint` | `;tp` | (opens colour picker) | — |
| market | `market` | `;m` | (opens market) | — |
| market | `install` | `;mi` | `m5tui market install {theme}` | `theme` |
| market | `publish` | `;mp` | `m5tui market publish` | (asks for description) |
| system | `wizard` | `;bw` | (runs first-boot wizard) | — |
| system | `doctor` | `;D` | (runs doctor) | — |
| system | `about` | `;??` | (opens about screen) | — |
| system | `quit` | `;qq` | (asks for confirmation) | — |
| system | `reboot` | `;qr` | `sudo reboot` (asks for confirmation) | — |

*(The list is the proposed default; small adjustments are expected as we use the device.)*

---

**End of COMMANDS.md.**
