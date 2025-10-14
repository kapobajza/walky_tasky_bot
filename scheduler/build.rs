use std::fs;
use std::path::Path;

fn main() {
    let root_dir_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory");
    let git_path = root_dir_path.join(".git");
    let hook_source_path = root_dir_path.join("scripts/pre-commit");

    println!("cargo:rerun-if-changed={}", hook_source_path.display());

    let hook_dest_path = git_path.join("hooks/pre-commit");

    if git_path.exists() {
        if let Ok(content) = fs::read_to_string(hook_source_path.clone()) {
            if let Err(e) = fs::write(hook_dest_path.clone(), content) {
                println!("cargo:warning=Failed to copy pre-commit hook: {}", e);
            } else {
                // Make the hook executable on Unix systems
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = fs::metadata(hook_dest_path.clone()) {
                        let mut perms = metadata.permissions();
                        perms.set_mode(0o755);
                        let _ = fs::set_permissions(hook_dest_path, perms);
                    }
                }
            }
        }
    } else {
        println!("cargo:warning=.git directory not found; skipping pre-commit hook installation");
    }
}
