mod utils;

use std::env;
use std::path::Path;
use dotenv::dotenv;
use autocontain::{check_github_repo, clone_repo, cleanup_repos, analyze_markdown};
use utils::{user_exit};

fn main() {
    dotenv().ok();
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found in .env.");

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: autocontain <github_repo_link>");
        std::process::exit(1);
    }

    let link = &args[1];
    let persist = args.len() > 2 && args[2] == "--persist";

    if !link.starts_with("https://github.com/") {
        println!("Not a Github repository link.");
        return;
    }

    match check_github_repo(link) {
        Ok(true) => {
            println!("Valid link: {}", link);

            if let Err(e) = clone_repo(link, persist) {
                eprintln!("Failed to clone repository: {}", e);
            }
            let repo_name = link.trim_end_matches('/').split('/').last().unwrap();
            let local_path = Path::new("source").join(repo_name);
            match analyze_markdown(&local_path, &openai_api_key) {
                Ok(analysis) => println!("Analysis:\n{}", analysis),
                Err(e) => eprintln!("Failed to analyze .md files: {}", e),
            }            
            user_exit();
            cleanup_repos();
        },
        Ok(false) => println!("Invalid link"),
        Err(e) => eprintln!("Error: {}", e),
    }
}
