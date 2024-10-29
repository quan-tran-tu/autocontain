use std::env;
use dotenv::dotenv;
use std::process;
use autocontain::{process_repository};
use autocontain::utils::user_exit;
use autocontain::repo::cleanup_repos;

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

    // Extract GitHub link and optional persist flag
    let link = &args[1];
    let persist = args.len() > 2 && args[2] == "--persist";

    // Validate GitHub link format
    if !link.starts_with("https://github.com/") {
        eprintln!("Invalid GitHub repository link.");
        process::exit(1);
    }

    // Process the repository using the modularized function
    if let Err(e) = process_repository(link, &openai_api_key, persist) {
        eprintln!("Error processing repository: {}", e);
    }
    
    user_exit();

    if !persist {
        cleanup_repos();
    }
}
