//! Ensures `assets/dashboard` exists so `rust-embed` can compile
//! even before the Preact UI has been built.
use std::fs;
use std::path::Path;

fn main() {
    let dir = Path::new("assets/dashboard");
    let index = dir.join("index.html");
    if !index.exists() {
        fs::create_dir_all(dir).expect("create assets/dashboard");
        fs::write(
            &index,
            r#"<!DOCTYPE html><html><body>
<p>Run <code>cd frontend && npm install && npm run build</code> then rebuild.</p>
</body></html>"#,
        )
        .expect("write placeholder index.html");
        println!("cargo:warning=assets/dashboard missing; embedded placeholder. Run `cd frontend && npm run build`.");
    }
    println!("cargo:rerun-if-changed=assets/dashboard");
    println!("cargo:rerun-if-changed=frontend/src");
}
