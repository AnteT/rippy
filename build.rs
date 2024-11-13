use std::env;
use chrono::Utc;

fn main() {
    // Get current date as YYYY-MM-DD
    let date_str = Utc::now().format("%Y-%m-%d").to_string();
    
    // Read version from environment variable
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "Unknown".to_string());
    
    // Format version with date
    let version_with_date = format!("v{} ({})", version, date_str);
    
    // Set the environment variable RELEASE_INFO
    println!("cargo:rustc-env=RELEASE_INFO={}", version_with_date);
}
