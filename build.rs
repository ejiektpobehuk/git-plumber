use built;

fn main() {
    // Generate built.rs with build information
    built::write_built_file().expect("Failed to acquire build-time information");
}
