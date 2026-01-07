//! Build script for soul-storage.
//!
//! This script ensures the crate is rebuilt when database migrations change.

fn main() {
    // Trigger rebuild when migrations change
    println!("cargo:rerun-if-changed=migrations");
}
