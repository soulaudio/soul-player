use colored::Colorize;

/// Print a section header
pub fn print_header(text: &str) {
    println!("\n{}", text.cyan().bold());
}

/// Print a success message with checkmark
pub fn print_success(text: &str) {
    println!("  {} {}", "✓".green(), text);
}

/// Print an error message with X
pub fn print_error(text: &str) {
    println!("  {} {}", "✗".red(), text);
}

/// Print a warning message
pub fn print_warning(text: &str) {
    println!("  {} {}", "⚠".yellow(), text);
}

/// Print an info message
pub fn print_info(text: &str) {
    println!("  {} {}", "ℹ".blue(), text);
}

/// Print a step message
pub fn print_step(text: &str) {
    println!("  {} {}", "→".cyan(), text);
}

/// Print a completion message
pub fn print_complete(text: &str) {
    println!("\n{} {}", "🎉".normal(), text.green().bold());
}
