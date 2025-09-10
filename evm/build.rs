fn main() {
    // Make cargo re-run the build script if .env changes
    println!("cargo:rerun-if-changed=.env");

    // Read from your .env file and set cargo environment variables
    if let Ok(env_content) = std::fs::read_to_string(".env") {
        for line in env_content.lines() {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let value = parts[1].trim();
                println!("cargo:rustc-env={}={}", key, value);
            }
        }
    }
}
