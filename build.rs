fn main() {
    // Tell cargo to re-run if credentials change so the binary is always fresh.
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=GOOGLE_CLIENT_SECRET");
}
