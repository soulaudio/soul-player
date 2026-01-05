fn main() {
    // Trigger rebuild when migrations change
    println!("cargo:rerun-if-changed=migrations");
}
