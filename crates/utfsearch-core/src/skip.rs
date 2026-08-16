use std::path::Path;

/// Directory names skipped at any depth (case-insensitive).
const ALWAYS: &[&str] = &[
    "$recycle.bin",
    "recycle.bin",
    "system volume information",
    "recovery",
    "windows.old",
    "$windows.~bt",
    "$windows.~ws",
    "config.msi",
    "$winreagent",
    "msocache",
    "onedrivetemp",
    "proc",
    "sys",
    "dev",
    // Language / package trees — huge and useless for document search.
    "node_modules",
    "bower_components",
    "jspm_packages",
    ".npm",
    ".yarn",
    ".pnpm",
    ".pnpm-store",
    ".next",
    ".nuxt",
    ".output",
    ".turbo",
    ".parcel-cache",
    ".venv",
    "venv",
    "virtualenv",
    ".virtualenv",
    "__pycache__",
    ".tox",
    ".nox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".eggs",
    "site-packages",
    "build",
    "dist",
    "target",
    "coverage",
    ".gradle",
    "vendor",
    "pods",
    ".dart_tool",
    ".git",
    ".hg",
    ".svn",
    ".idea",
    ".vs",
    ".cache",
];

/// Only skipped when they sit directly on a drive root (`C:\Windows`).
const VOLUME_CHILD: &[&str] = &[
    "windows",
    "program files",
    "program files (x86)",
    "programdata",
    "documents and settings",
    "perflogs",
    "intel",
    "amd",
    "nvidia",
];

const SKIP_FILES: &[&str] = &[
    "pagefile.sys",
    "hiberfil.sys",
    "swapfile.sys",
    "dumpstack.log.tmp",
    "thumbs.db",
    "desktop.ini",
];

pub fn skip_dir(root: &Path, path: &Path) -> bool {
    if path == root {
        return false;
    }
    let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    let key = name.to_ascii_lowercase();
    if ALWAYS.contains(&key.as_str()) || key.ends_with(".egg-info") {
        return true;
    }
    if is_volume_child(root, path) && VOLUME_CHILD.contains(&key.as_str()) {
        return true;
    }
    has_system_attr(path)
}

pub fn skip_file(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
        if SKIP_FILES.contains(&name.to_ascii_lowercase().as_str()) {
            return true;
        }
    }
    has_system_attr(path)
}

fn is_volume_child(root: &Path, path: &Path) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    parent == root && is_volume_root(root)
}

fn is_volume_root(path: &Path) -> bool {
    let slim = crate::paths::slim(path);
    slim.parent().is_none()
        || slim
            .components()
            .all(|c| matches!(c, std::path::Component::Prefix(_) | std::path::Component::RootDir))
}

fn has_system_attr(path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
        std::fs::metadata(path)
            .map(|m| m.file_attributes() & FILE_ATTRIBUTE_SYSTEM != 0)
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn recycle_always_skipped() {
        let root = PathBuf::from(r"D:\data");
        assert!(skip_dir(&root, &root.join("$Recycle.Bin")));
        assert!(skip_dir(&root, &root.join("docs").join("System Volume Information")));
        assert!(!skip_dir(&root, &root.join("docs")));
        assert!(!skip_dir(&root, &root.join("Windows")));
        assert!(skip_dir(&root, &root.join("proj").join("node_modules")));
        assert!(skip_dir(&root, &root.join("proj").join(".venv")));
        assert!(skip_dir(&root, &root.join("proj").join("build")));
        assert!(skip_dir(&root, &root.join("proj").join("target")));
        assert!(!skip_dir(&root, &root.join("proj").join("src")));
    }

    #[test]
    fn windows_skipped_only_on_drive() {
        let drive = PathBuf::from(r"C:\");
        assert!(skip_dir(&drive, &drive.join("Windows")));
        assert!(skip_dir(&drive, &drive.join("Program Files")));
    }
}
