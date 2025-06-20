/// Print a banner message with decorative formatting
pub fn print_banner(message: &str) {
    println!("🚀 {}", message);
}

/// Print a success message with green formatting
pub fn print_success(message: &str) {
    println!("✅ {}", message);
}

/// Print an error message with red formatting
pub fn print_error(message: &str) {
    eprintln!("❌ {}", message);
}

/// Print an informational message with blue formatting
pub fn print_info(message: &str) {
    println!("ℹ️  {}", message);
} 