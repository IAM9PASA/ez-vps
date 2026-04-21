use console::style;

pub fn print_action(message: &str) {
    println!("{} {}", style(">").cyan().bold(), message);
}

pub fn print_success(message: &str) {
    println!("{} {}", style("+").green().bold(), message);
}

pub fn print_kv(key: &str, value: &str) {
    println!("  {} {}", style(format!("{key}:")).dim(), value);
}
