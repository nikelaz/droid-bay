# Skill setup

A standalone, full-screen terminal application for installing the repository's skills. The interface uses Ratatui's immediate rendering and Crossterm for cross-platform keyboard and mouse input. It has a dark navy palette, cyan and violet accents, rounded panels, a scrollable skill list, and a skill details pane on wide terminals. Windows, Linux, and macOS are supported. Windows uses the operating system's PowerShell to create directory junctions; Unix uses symbolic links.

```sh
cargo build --release --manifest-path setup-tui/Cargo.toml
```

Run `setup-tui/target/release/setup-tui` (`setup-tui.exe` on Windows). Skills and their supporting files are embedded at build time; the executable does not need the repository at runtime.

The default installation root is your home directory. Use `setup-tui --root PATH` for a project or another directory. Destinations are `<root>/.agents/skills/<skill>` and `<root>/.claude/skills/<skill>`.

Use Up/Down (or `j`/`k`) to move through skills and Space or Enter to toggle selection. Tab and Shift+Tab move between skills, destinations, and actions. Left/Right chooses a destination or action; Space or Enter activates it. Mouse clicks toggle skills and destinations or activate action buttons; the wheel scrolls the skill list. The terminal must be at least 64 columns by 22 rows; the details pane appears at 108 columns.

| Key | Action |
| --- | --- |
| `a` | Select all or none |
| `g` / `c` | Toggle Agent / Claude destinations |
| `i` / `u` | Activate the combined Install / Update action |
| `r` | Uninstall selected skills; Enter or `y` confirms, Esc or `n` cancels |
| `l` | Open activity log; arrows or mouse wheel scroll, Esc closes |
| F5 | Refresh installation status |
| `q`, Esc, Ctrl+C | Quit; an active operation finishes before exiting |

The primary button follows the selected skills at the selected destinations: `Install` when none are installed, `Update` when all are installed, and `Install & Update` for a mixture. It installs missing copies and updates existing copies. With no skills or destinations selected, the label defaults to `Install`. `Uninstall` is always available.

Installation runs in the background with a progress indicator and a per-skill activity log. Updates replace the entire installed skill folder, including local edits. Removing only the Agent copy can leave a Claude link dangling; select both locations to remove both.

Selecting both destinations installs the Agent copy and makes each Claude skill a link to that copy. An existing independent Claude copy is replaced with a link. Selecting only Claude installs independent copies. Removal deletes links without following them.

The list shows the bundled version and each destination's installed version. A green check means current, an amber arrow means a different version, a dash means absent, and `Unknown` means a missing or unreadable version file. A diagonal arrow marks a link. Versions are compared for equality, so updates can also install an older bundled version. Bump a skill's `VERSION.txt` when changing its content, then rebuild the executable. Each skill directory must contain `SKILL.md` and a nonempty `VERSION.txt`.

Installation stages the replacement beside the destination, renames the previous installation to a backup, and restores it if activation fails. Errors are reported per skill; completed installations remain installed.
