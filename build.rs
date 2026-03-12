use std::fs;

fn main() {
    // Skip README generation on docs.rs
    if std::env::var("DOCS_RS").as_deref() == Ok("1") {
        return;
    }

    /*
    Compose README.md from the building blocks in docs/readme_parts,
    interleaving image links in between. Finally, all {{VERSION}} placeholders
    are replaced by the actual version read from Cargo.toml.
     */

    let mut readme =
        fs::read_to_string("docs/readme_parts/links.md").expect("Failed to read template");
    readme.push('\n');
    readme.push_str(
        &fs::read_to_string("docs/readme_parts/circle.svg.md").expect("Failed to read template"),
    );
    readme.push_str("\n\n![](https://raw.githubusercontent.com/StefanMathis/akima_spline/refs/heads/main/docs/img/circle.svg \"Circle from code\")\n\n");

    readme.push_str(
        &fs::read_to_string("docs/readme_parts/end.md").expect("Failed to read template"),
    );

    let readme = readme.replace(
        "{{VERSION}}",
        &std::env::var("CARGO_PKG_VERSION").expect("version is available in build.rs"),
    );
    let _ = fs::write("README.md", readme);
}
