//! Resolve assets beside the deployed binary before any development checkout.
//! Cargo can reuse an unchanged binary across release directories; compiled-in
//! manifest paths therefore must never select an older production release.
use std::path::{Path, PathBuf};

fn from_roots(relative: &Path, roots: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    roots.into_iter().map(|root| root.join(relative)).find(|path| path.is_file())
}

pub(crate) fn file(relative: &str) -> Option<PathBuf> {
    let executable_root = std::env::current_exe().ok()
        .and_then(|p| p.parent()?.parent().map(Path::to_path_buf));
    let roots = executable_root.into_iter()
        .chain(std::env::current_dir().ok())
        .chain([PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")]);
    from_roots(Path::new(relative), roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deployed_root_precedes_compiled_checkout() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let result = from_roots(Path::new("web/index.html"), [root.clone(), root.join("missing")]).unwrap();
        assert_eq!(result, root.join("web/index.html"));
    }
}
