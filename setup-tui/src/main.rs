use std::{
    env, fs,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
};

mod ui;

struct Skill {
    name: &'static str,
    version: &'static str,
    files: &'static [(&'static str, &'static [u8])],
}

include!(concat!(env!("OUT_DIR"), "/skills.rs"));

fn exists(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

fn remove(path: &Path) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if is_link(&metadata) {
        #[cfg(windows)]
        return fs::remove_dir(path);
        #[cfg(not(windows))]
        return fs::remove_file(path);
    }
    if !metadata.is_dir() || !path.join("SKILL.md").is_file() {
        return Err(io::Error::other(format!(
            "Refusing to remove a non-skill directory: {}",
            path.display()
        )));
    }
    fs::remove_dir_all(path)
}

fn create_link(target: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, destination)
    }
    #[cfg(windows)]
    {
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command",
                "$ErrorActionPreference = 'Stop'; New-Item -ItemType Junction -Path $env:DROID_BAY_LINK -Target $env:DROID_BAY_TARGET | Out-Null"])
            .env("DROID_BAY_LINK", destination)
            .env("DROID_BAY_TARGET", shell_path(target))
            .output()?;
        if output.status.success() {
            Ok(())
        } else {
            Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ))
        }
    }
}

#[cfg(windows)]
fn shell_path(path: &Path) -> PathBuf {
    use std::path::{Component, Prefix};
    let mut components = path.components();
    let mut result = match components.next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:\\", char::from(drive))),
            Prefix::VerbatimUNC(server, share) => {
                let mut result = PathBuf::from(r"\\");
                result.push(server);
                result.push(share);
                result
            }
            _ => return path.to_path_buf(),
        },
        _ => return path.to_path_buf(),
    };
    for component in components {
        if component != Component::RootDir {
            result.push(component.as_os_str());
        }
    }
    result
}

fn replace(skill: &Skill, destination: &Path, link: Option<&Path>) -> io::Result<()> {
    let parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("Missing install directory"))?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(".{}.setup-{}", skill.name, std::process::id()));
    let backup = parent.join(format!(".{}.backup-{}", skill.name, std::process::id()));
    if exists(&staging)? || exists(&backup)? {
        return Err(io::Error::other(format!(
            "Staging or backup directory already exists for {}",
            skill.name
        )));
    }
    let prepared = if let Some(target) = link {
        create_link(target, &staging)
    } else {
        fs::create_dir(&staging)?;
        (|| {
            for (relative, bytes) in skill.files {
                let file = staging.join(relative);
                if let Some(parent) = file.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(file, bytes)?;
            }
            Ok(())
        })()
    };
    if let Err(error) = prepared {
        if link.is_none() {
            fs::remove_dir_all(&staging)?;
        }
        return Err(error);
    }
    let had_previous = exists(destination)?;
    if had_previous {
        let metadata = fs::symlink_metadata(destination)?;
        if !is_link(&metadata) && (!metadata.is_dir() || !destination.join("SKILL.md").is_file()) {
            remove(&staging)?;
            return Err(io::Error::other(format!(
                "Refusing to replace a non-skill directory: {}",
                destination.display()
            )));
        }
        if let Err(error) = fs::rename(destination, &backup) {
            remove(&staging)?;
            return Err(error);
        }
    }
    if let Err(error) = fs::rename(&staging, destination) {
        if had_previous {
            fs::rename(&backup, destination)?;
        }
        remove(&staging)?;
        return Err(error);
    }
    if had_previous {
        remove(&backup)?;
    }
    Ok(())
}

enum InstalledStatus {
    Absent,
    Installed {
        version: Option<String>,
        linked: bool,
    },
    Error(String),
}

fn status(path: &Path) -> InstalledStatus {
    let linked = match fs::symlink_metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return InstalledStatus::Absent,
        Err(error) => return InstalledStatus::Error(error.to_string()),
        Ok(metadata) => is_link(&metadata),
    };
    let version = fs::read_to_string(path.join("VERSION.txt"))
        .ok()
        .map(|version| version.trim().to_owned())
        .filter(|version| !version.is_empty());
    InstalledStatus::Installed { version, linked }
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Install,
    Update,
    InstallAndUpdate,
    Remove,
}

fn apply_skill(
    skill: &Skill,
    root: &Path,
    action: Action,
    use_agents: bool,
    use_claude: bool,
) -> io::Result<()> {
    let agents = root.join(".agents/skills");
    let claude = root.join(".claude/skills");
    let agent_path = agents.join(skill.name);
    let claude_path = claude.join(skill.name);
    let shared_directory = agents
        .canonicalize()
        .ok()
        .zip(claude.canonicalize().ok())
        .is_some_and(|(left, right)| left == right);
    if action == Action::Remove {
        if use_claude && !(use_agents && shared_directory) {
            remove(&claude_path)?;
        }
        if use_agents {
            remove(&agent_path)?;
        }
    } else {
        if use_agents && (action != Action::Install || !exists(&agent_path)?) {
            replace(skill, &agent_path, None)?;
        }
        if use_claude && !(use_agents && shared_directory) {
            let linked = use_agents
                && claude_path
                    .canonicalize()
                    .ok()
                    .zip(agent_path.canonicalize().ok())
                    .is_some_and(|(left, right)| left == right);
            if action != Action::Install || !exists(&claude_path)? || use_agents && !linked {
                replace(
                    skill,
                    &claude_path,
                    if use_agents { Some(&agent_path) } else { None },
                )?;
            }
        }
    }
    Ok(())
}

fn run() -> io::Result<()> {
    let mut arguments = env::args_os().skip(1);
    let root = match arguments.next() {
        Some(argument) if argument == "--help" || argument == "-h" => {
            println!("setup-tui [--root DIRECTORY]\n\nInstall bundled skills into DIRECTORY/.agents/skills and DIRECTORY/.claude/skills.\nDefault DIRECTORY is your home folder. Navigate with arrows, Tab, and Space.");
            return Ok(());
        }
        Some(argument) if argument == "--root" => PathBuf::from(
            arguments
                .next()
                .ok_or_else(|| io::Error::other("--root requires a directory"))?,
        ),
        Some(_) => return Err(io::Error::other("Usage: setup-tui [--root DIRECTORY]")),
        None => PathBuf::from(
            env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok_or_else(|| {
                io::Error::other("Home directory unavailable; use --root DIRECTORY")
            })?,
        ),
    };
    if arguments.next().is_some() {
        return Err(io::Error::other("Usage: setup-tui [--root DIRECTORY]"));
    }
    fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    #[cfg(windows)]
    let root = shell_path(&root);
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other("An interactive terminal is required"));
    }
    ui::run(root)
}

fn main() {
    if let Err(error) = run() {
        eprintln!("setup-tui: {error}");
        std::process::exit(1);
    }
}
