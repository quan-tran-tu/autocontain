use std::process;
pub fn print_usage_and_exit() {
    eprintln!("Usage:");
    eprintln!(" cargo run -- <github_repo_link> [--persist] [--depth]");
    eprintln!(" cargo run -- rm <repo_link_or_name>");
    process::exit(1);
}