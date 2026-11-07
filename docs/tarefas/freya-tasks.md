# Port Expressa Aula GUI to Freya

Status: planned. **Linux and Windows are both product targets.** GTK `expressa-aula` keeps shipping until Freya reaches local-editor + run parity.

## Platforms

Daily work and stage acceptance run on **Linux**. A full **Windows** pass happens after the port is functionally done (Stage 11), not after every slice.

The binary must stay portable from the first crate: same UI, shortcuts, and student flows on Linux and Windows. Use `dirs` / `cfg(unix)` / `cfg(windows)` for paths, signals, and install layout. Do not call Windows-only APIs in the Linux path, and do not assume GTK portals exist on Windows.

| | Linux (dev + ship) | Windows (ship; test at the end) |
|---|---|---|
| Renderer | Vulkan, then OpenGL, then software | OpenGL, then software (Vulkan opt-in) |
| Paths | XDG config/data, Documents | `%APPDATA%`, `%LOCALAPPDATA%`, Documents |
| Package | `.deb` without libgtk / gtksourceview | Inno/zip without GTK DLLs |
| Verify | every stage, this machine | one native walkthrough when Stages 0–10 are done |

Document `FREYA_RENDERER=software` for lab PCs without a usable GPU. Do not require Vulkan.

## Does a staged port make sense?

Yes. Start with the text editor, then add features in slices that a student can already use.

Aula is an IDE. The editor is the product. Freya 0.5 ships `freya-code-editor` (rope buffer, tree-sitter highlighting, the same stack as Valin). That is why Freya is the right custom-renderer choice here. The same Skia UI also removes the GTK sysroot from Windows packaging, which is why Windows remains a first-class target even though you will not boot Windows until the end.

Do this as a **second UI crate**, not an in-place rewrite of `aula/ui`. Keep the GTK binary until the Freya one can open a `.lep`, type Portuguese, save, and run with F5.

Stage 0 is a kill-gate **on Linux**: if ABNT dead keys or the code editor are unusable here, stop and reassess. Do not start menus, RPC, or graphics before that gate passes. Re-check IME on Windows in Stage 11 (winit IME differs from IBus/GTK compose).

## Product that must survive

The student-facing app stays **Expressa Aula**. Same shortcuts and Portuguese UI as `aula/README.md`.

Keep unchanged (no Freya):

- Language VM (`expressa`)
- `expressa-aula-proto`, `expressa-aula-server`, `expressa-aula-storage`
- Exec / debug protocol (`DebugHook`, `StopHook`, `run_with_hosts`)
- Native docs (`NATIVAS`, `docs/nativas/*.lep`)
- `expressa-tutorial` (spawn from Ajuda; it is its own GTK app for now)

Rewrite (Freya shell):

- Window, menus, theme, file tree, tabs, editor, console, REPL, Ajuda, debugger pane, cartesiano / `tela_grafica` windows, dialogs, Linux `.deb` and Windows installer chrome

Reuse by extracting GTK out of logic (do this as you touch each area, not as a big-bang `aula/core`):

| Logic today | GTK coupling | Freya plan |
|---|---|---|
| `execution_state.rs` | none | use as-is |
| `indent.rs` | SourceBuffer | split: pure indent on `&str` / rope, Freya applies it |
| `lang.rs` / `analysis.rs` | light | move toward proj/07; Freya calls the same API |
| `prefs.rs` | glib paths | same keys; `dirs` (XDG on Linux, `%APPDATA%` on Windows) |
| `store.rs` / `rpc.rs` | mostly free | reuse |
| `documentos.rs`, `exec.rs`, `console.rs`, `assist.rs` | GTK widgets | new Freya views on the same models |

## Crate layout

```text
aula/ui-freya/       crate expressa-aula-freya (GTK aula/ui stays)
  Cargo.toml         freya = "0.5" features: winit, code-editor, sdk-tokio
  src/main.rs        launch + window
  src/editor.rs      code editor, tabs, indent, search
  src/shell.rs       menu, toolbar, status, theme, shortcuts
  src/projeto.rs     tree, local folder, exemplos
  src/execucao.rs    F5/F6, console, leia, Parar
  src/ajuda.rs       F1, catalog, completion
  src/debug.rs       gutter, vars, stack
  src/grafico.rs     cartesiano + tela_grafica hosts
```

Binary name during dual-ship: `expressa-aula-freya` (`expressa-aula-freya.exe` on Windows).
After cutover: this crate becomes `expressa-aula` and GTK `aula/ui` is archived.

Freya features to turn on as needed: `code-editor`, `sdk-tokio` (Exec on a Tokio runtime), `devtools` in debug builds, `plot` only if cartesiano uses plotters. Leave `terminal` / PTY off; Aula’s console is a bounded text pane, not a system shell.

## Rules for every stage

1. One stage = one reviewable slice. You can run the binary **on Linux** and see the new thing.
2. Headless tests for pure logic (`indent`, `lang`, `execution_state`, console ANSI). Freya UI tests via `freya-testing` where they are cheap. Manual Linux pass for IME, menus, and file dialogs on every stage. Windows walkthrough only in Stage 11.
3. Do not port a GTK widget 1:1. Reimplement the **behavior** in Freya components.
4. Keep paths, dialogs, and spawn (`expressa-tutorial`) behind a small platform layer so Linux and Windows stay in one crate.
5. GTK Aula remains the classroom release until Stage 5 (run) is done on Linux.

---

## Stage 0 — Editor spike on Linux (kill-gate)

**Goal:** prove Freya can host a code editor a Brazilian student can type into, on this Linux machine.

Work:

- Linux build (Freya deps: clang, cmake, fontconfig, EGL/GL, Wayland). rust-skia prebuilts.
- Window + `freya-code-editor` with a rope buffer, line numbers, caret, undo.
- Monospace font that covers Portuguese (`ã é ç õ`).
- IME: ABNT dead keys (`~` + `a` → `ã`, `´` + `e` → `é`, `ç`). Composition must not drop or duplicate characters (IBus / compose on Linux).
- Clipboard paste of mixed Portuguese + code.
- Note binary size and whether Vulkan/OpenGL starts; try `FREYA_RENDERER=software`.
- Keep the crate compiling with `cfg(windows)` path stubs so Stage 11 is not a rewrite. Do not require a Windows box for this stage.

Acceptance:

- [ ] `cargo run -p expressa-aula-freya` on Linux opens a window in a few seconds.
- [ ] You can type `função`, `não`, `início` without fighting the editor.
- [ ] Undo/redo and paste work.
- [ ] Software fallback is documented if GPU init fails.

**Stop** if IME or the editor is unusable on Linux. That is cheaper than discovering it at Stage 3.

First visible result: a blank Freya window with a working editor. This is the “see Freya” milestone.

---

## Stage 1 — App shell

**Goal:** an empty IDE chrome around the editor.

Work:

- Workspace member `aula/freya`, `expressa-aula-freya` binary, Linux CI job (build + test, no GTK).
- Layout: menubar (Arquivo / Executar / Depurar / Ver / Ajuda), toolbar row, status bar (`linha:coluna`, project path later).
- Claro / Escuro theme (prefs file compatible with GTK keys: `fonte`, `tema`, `pasta`, `arquivo`).
- Shortcuts skeleton: Ctrl+S, Ctrl+Q, Ctrl+= / Ctrl+- / Ctrl+0, F5–F11 reserved even if no-ops.
- Maximize on start. Editor takes most of the window.

Acceptance:

- [x] Window looks like an IDE, not a demo counter.
- [x] Theme switch repaints chrome + editor.
- [x] Prefs round-trip on Linux (`~/.config/expressa-aula/prefs` via `dirs`). Same keys as GTK Aula.

---

## Stage 2 — Text editor as a document

**Goal:** open, edit, save `.lep` files. This is the first *Aula* slice.

Work:

- One tab, then multiple tabs (`Ctrl+N` / `Ctrl+W`). Dirty `●` on the tab.
- Open file / save / save as (portable dialogs: `rfd` or Freya platform; xdg portal on Linux, native dialogs on Windows).
- Confirm discard on close with unsaved text.
- Status `linha:coluna`. **Ver → Ir para linha** (Ctrl+G).
- Electric-indent later (Stage 3); here only spaces-for-Tab is enough.
- Autosave scratch every 5 s (reuse storage ideas from `aula/ui` recovery; wire UI in Stage 10 if needed). Minimum: dirty flag + confirm.

Acceptance:

- [x] Open `tutorial/exemplos/*.lep`, edit, save, reopen.
- [x] Two tabs, close with prompt.
- [x] Ctrl+Z / Ctrl+Shift+Z per tab.

---

## Stage 3 — Expressa editing

**Goal:** the buffer behaves like today’s editor for typing code.

Work:

- Syntax highlight: `freya-code-editor` wants a **tree-sitter** grammar (`EditorLanguage { language, highlights_query }`). There is no `tree-sitter-expressa` yet. Add a small grammar covering keywords (both accented and ASCII forms from `KEYWORDS`), comments, strings, numbers, `:ident`. Keep `expressa.lang` for GTK until cutover.
- Indent: extract the pure algorithm from `indent.rs` (today it reads `SourceBuffer`). Apply on Enter, `}` / `fim`, Tab / Shift+Tab, Ctrl+Alt+\\.
- Ctrl+/ comment toggle, Ctrl+D duplicate line, Alt+↑/↓ move, matching `inicio`/`fim` / `{`/`}` highlight.
- Ctrl+F search (Esc closes).
- Underline parse errors while typing; message on the status bar (analysis can stay on a worker, proj/06–07).

Acceptance:

- [ ] `poema.lep` with `"início: …"` does not confuse indent.
- [ ] Top-level `escreva(...)` does not indent the next line.
- [ ] Keyword list matches `lang.rs` / `KEYWORDS` tests.
- [ ] Search finds accented identifiers.

---

## Stage 4 — Local project

**Goal:** a folder of files, like home use without a classroom server.

Work:

- File tree (hide dot dirs, `target`, `node_modules`; refuse huge trees).
- Arquivo → Local → Abrir projeto… ; recent projects (10).
- Context menu: new file/folder, rename, delete. Nameless files get `.lep`.
- First-run copy of shipped `exemplos` into Documents/Expressa Aula/exemplos (`copy_missing`; XDG documents on Linux).
- Restore last folder + last file from prefs.
- Status bar shows project path (`~/...` display).

Acceptance:

- [ ] Open Documents/Expressa Aula/exemplos, click a file, it opens in a tab.
- [ ] New file appears in the tree and on disk.
- [ ] Restart restores folder and file.

---

## Stage 5 — Run (F5) and console

**Goal:** a student can Rodar a program. This is the first *teaching* slice.

Work:

- In-process `run_with_hosts` (same as GTK Aula). No `expressa` CLI spawn.
- Execution workspace copy under XDG data on Linux and `%LOCALAPPDATA%` on Windows (existing storage rules).
- Console pane: last 5000 lines; ANSI SGR (`pinte` / `fundo` / `negrito`); `quadro` / `cls` / CUP like `console.rs`.
- `leia`: type in the console + Enter (`<<<EXPRESSA-LEIA>>>`). UI stays live.
- Parar via `StopHook`. Args row → `argumentos[1]…`. stderr in red.
- Click `em arquivo:linha` jumps to the tab.
- F5 uses unsaved tab text (current Aula rule).

Acceptance:

- [ ] F5 on `escreva("olá")` prints in the console.
- [ ] `leia("nome:")` accepts a line without freezing the window.
- [ ] Infinite `enquanto verdadeiro` stops on Parar.
- [ ] `conway_jogo_da_vida.lep` runs until Parar (no classroom timeout).

Do not wait for the debugger to ship a teaching build. After Stage 5, Freya Aula is usable at home on Linux.

---

## Stage 6 — Help and completion

**Goal:** F1 and Ctrl+Space match GTK Aula.

Work:

- F1 on a name → ficha (`lookup_builtin_doc` / `help_markup`). F1 off a name → Ajuda catalog window (sibling, not modal).
- Ctrl+Space: natives + current file + `importe`; choosing `pinte` inserts `importe "tela"` if needed.
- Signature hint inside a call. Esc dismisses.
- F12 / Ctrl+click go to definition.
- Ajuda → Tutoriais spawns `expressa-tutorial` next to the binary (`expressa-tutorial.exe` on Windows).

Acceptance:

- [ ] F1 on `escreva` shows the same ficha as the REPL.
- [ ] Completion lists `matriz::zeros` after `importe "matriz"`.
- [ ] Ajuda window can sit beside the editor while typing.

---

## Stage 7 — Debugger

**Goal:** F6 and the right-hand panel.

Work:

- Layout: tree | editor+console | debug (Observados / Variáveis / Pilha). F7/F8 toggle.
- F6 starts debug; F9 / gutter click toggles breakpoint; F10 / F11 / Shift+F11.
- Current line highlight; execution snapshot tab (read-only source that ran).
- Breakpoints track inserts/deletes (proj/05). Grey hollow mark = non-executable line.
- Reuse `ChannelDebugger` / `DebugPaused`; only the view is new.

Acceptance:

- [ ] Break on first line, step, see locals.
- [ ] Click stack frame opens the file and line.
- [ ] Editing during a run does not change the running snapshot.

---

## Stage 8 — REPL

**Goal:** Ver → REPL.

Work:

- Panel with streaming output, error color, Parar, Reiniciar, 5000-line / 256 KiB bound (proj/09).
- Session variables persist across commands; `saia()` ends the session.

Acceptance:

- [ ] `2+2` then `x = 1` then `x` works.
- [ ] A tight `escreva` loop yields to the UI; Parar recovers the prompt.

---

## Stage 9 — Graphics windows

**Goal:** `mostre()` for cartesiano and `tela_grafica`.

Work:

- Cartesiano: Freya canvas (or `plot` feature) docked above the console; Destacar / Acoplar like Ajuda. Same y-up notebook mapping.
- `tela_grafica`: one sibling window per `nova()`; mouse/key → `evento()`; Parar / F5 close; closed id stays closed until limpar (current lifecycle).
- Do not mix CLI minifb and Aula hosts beyond the shared event/lifecycle types.

Acceptance:

- [ ] `eixos` + `plotar` + `mostre()` updates the same window.
- [ ] Pixel `nova()` click returns in `evento()`.
- [ ] Closing X does not recreate the window until the next F5.

---

## Stage 10 — Classroom and durability

**Goal:** remote Turma, recovery, conflicts, sync. Last functional stage.

Work:

- Conectar ao servidor, student folder, project list, sync window (pending uploads + unsaved edits).
- Safe save + conflict dialogs (proj/02).
- Recovery of scratch buffers (proj/01).
- Operations popover (cancel background open/save/prepare).
- Binary assets: follow proj/04; do not invent a new remote binary protocol here.

Acceptance:

- [ ] Home: loopback server, aluno `local`, same as GTK.
- [ ] Unsaved recovery after kill.
- [ ] Conflict: Comparar / Recarregar / Salvar como / Substituir.

---

## Stage 11 — Packaging, Windows pass, cutover

**Goal:** ship Linux and Windows without GTK. This is the first time the Freya UI is walked through on Windows.

Work:

- Linux `.deb` (extend `empacotar-deb.sh`): `expressa-aula-freya`, server, tutorial, exemplos. **No** `libgtk-4` / `libgtksourceview-5` for the Freya binary. Record size vs the GTK `.deb`.
- Native Windows build on a Windows machine (MSVC, VS 2022 C++ workload, rust-skia). No Linux→Windows cross.
- Inno Setup (or successor of `criar-instalador.ps1`): `expressa-aula-freya.exe`, server, tutorial, exemplos, VC++ CRT. **No** GTK DLLs.
- Full Windows checklist: ABNT IME, file dialogs, F5, `leia`, theme, exemplos path, installer.
- Optional software-renderer note in README for both OSes.
- When Stages 0–10 match `aula/README.md` on Linux **and** the Windows walkthrough passes: rename binary to `expressa-aula`, stop shipping GTK UI, keep GTK crate in-tree one release if needed.

Acceptance:

- [ ] Linux: install the Freya `.deb` on a clean-ish box (or this machine) and F5 an exemplo.
- [ ] Windows: clean VM or native box installs and F5 an exemplo; IME types `função` / `não`.
- [ ] Zip/deb size and first-launch time recorded on both.
- [ ] `aula/README.md` documents the Freya binary and the GTK sunset.

---

## Suggested first PRs

| PR | Stage | What lands |
|---|---|---|
| 1 | 0 | `aula/freya` crate, empty editor window, Linux build notes |
| 2 | 0 | IME / font / software-renderer checklist on Linux (can be the same PR if small) |
| 3 | 1 | Shell, theme, prefs |
| 4 | 2 | Tabs, open/save |
| 5 | 3 | tree-sitter-expressa + indent extraction |
| 6 | 4 | Project tree |
| 7 | 5 | Exec + console + leia |

After PR 7 the app is worth daily use on Linux. Debugger, help, graphics, and Turma follow the table above. Windows is the Stage 11 pass.

## Testing matrix (every stage that adds UI)

- `cargo test -p expressa-aula-freya` on Linux (logic + `freya-testing` where used)
- `cargo clippy -p expressa-aula-freya --all-targets -- -D warnings`
- Manual Linux: IME, file dialog, F5 (from Stage 5), theme
- GTK Aula tests stay green; do not regress the shipping IDE
- Manual Windows: only Stage 11 (IME, dialogs, F5, installer)

## Out of scope until cutover

- Replacing `expressa-tutorial` with Freya
- Android / Web Freya targets
- PTY terminal instead of the console pane
- Dropping GtkSourceView `expressa.lang` (still needed by GTK Aula and `.deb`)
- Cross-compiling Freya from Linux to Windows
