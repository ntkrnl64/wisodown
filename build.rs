use std::fs;
use std::path::Path;

fn main() {
    let dist = Path::new("frontend/dist");
    if !dist.exists() {
        fs::create_dir_all(dist).expect("failed to create frontend/dist");
        fs::write(
            dist.join("index.html"),
            "<html><body>Frontend not built. Run the frontend build first.</body></html>",
        )
        .expect("failed to write placeholder index.html");
    }
}
