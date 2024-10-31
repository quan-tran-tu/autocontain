use std::env;
use dotenv::dotenv;
use std::process;
use autocontain::{process_repository, run_menu};

fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Retrieve OpenAI API key from environment variables
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found in .env.");

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: autocontain <github_repo_link> [--persist]");
        process::exit(1);
    }

    let link = &args[1];

    // Default values
    let mut persist = false;
    let mut depth = 0;

    for arg in &args[2..] {
        match arg.as_str() {
            "--persist" => persist = true,
            _ if arg.starts_with("--depth=") => {
                if let Some(value) = arg.strip_prefix("--depth=") {
                    depth = value.parse::<usize>().unwrap_or(0);
                }
            }
            _ => println!("Warning: Unrecognized argument {}", arg),
        }
    }

    // Validate GitHub link format
    if !link.starts_with("https://github.com/") {
        eprintln!("Invalid GitHub repository link.");
        process::exit(1);
    }

    let (repo_name, local_path, scripts_path) = process_repository(link, &openai_api_key, persist, depth)
        .expect("Failed to process repository.");

    run_menu(persist, &local_path, &scripts_path);
}
