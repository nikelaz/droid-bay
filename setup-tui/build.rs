use std::{env, fs, io, path::Path};

fn collect_files(directory: &Path, root: &Path, output: &mut String) -> io::Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_files(&path, root, output)?;
        } else if entry.file_type()?.is_file() {
            let relative = path.strip_prefix(root).map_err(io::Error::other)?;
            output.push_str(&format!(
                "({:?}, include_bytes!({:?})),\n",
                relative.to_string_lossy().replace('\\', "/"),
                path.to_string_lossy()
            ));
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR").map_err(io::Error::other)?)
        .join("../skills")
        .canonicalize()?;
    println!("cargo:rerun-if-changed={}", root.display());
    let mut entries = fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut output = String::from("static SKILLS: &[Skill] = &[\n");
    for entry in entries {
        let path = entry.path();
        if !entry.file_type()?.is_dir() || !path.join("SKILL.md").is_file() {
            continue;
        }
        let version = fs::read_to_string(path.join("VERSION.txt"))?;
        if version.trim().is_empty() {
            return Err(io::Error::other(format!(
                "Empty version: {}",
                path.display()
            )));
        }
        output.push_str(&format!(
            "Skill {{ name: {:?}, version: {:?}, files: &[\n",
            entry.file_name().to_string_lossy(),
            version.trim()
        ));
        collect_files(&path, &path, &mut output)?;
        output.push_str("] },\n");
    }
    output.push_str("];\n");
    let destination = env::var("OUT_DIR").map_err(io::Error::other)?;
    fs::write(Path::new(&destination).join("skills.rs"), output)
}
