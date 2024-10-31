mod utils;
mod repo;

use std::env;
use dotenv::dotenv;
use std::process;
use autocontain::{process_repository, run_menu};
use utils::print_usage_and_exit;
use repo::{remove_repo, get_all_repos};


fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Retrieve OpenAI API key from environment variables
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found in .env.");

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage_and_exit();
    }

    match args[1].as_str() {
        "rm" => {
            let repo_name = &args[2];
            remove_repo(repo_name);
        }
        "run" => {
            let link = &args[2];
            // Validate GitHub link format
            if !link.starts_with("https://github.com/") {
                eprintln!("Invalid GitHub repository link.");
                process::exit(1);
            }
            // Default values
            let mut persist = false;
            let mut depth = 0;

            for arg in &args[3..] {
                match arg.as_str() {
                    "--persist" => persist = true,
                    _ if arg.starts_with("--depth=") => {
                        if let Some(value) = arg.strip_prefix("--depth=") {
                            depth = value.parse::<usize>().unwrap_or(0);
                        }
                    }
                    _ => {
                        println!("Warning: Unrecognized argument {}", arg);
                        print_usage_and_exit();
                    },
                }
            }

            let (repo_name, local_path, scripts_path) = process_repository(link, &openai_api_key, persist, depth)
                .expect("Failed to process repository.");

            run_menu(persist, &local_path, &scripts_path);
        }
        "list" => {
            get_all_repos();
        }
        _ => {
            eprintln!("Invalid argument '{}'", args[1]);
            print_usage_and_exit();
        }
    }
}
