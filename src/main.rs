use std::env;
use dotenv::dotenv;
use std::process;
use autocontain::{process_repository, run_menu};
use autocontain::utils::print_usage_and_exit;
use autocontain::repo::{remove_repo, get_all_repos};


use autocontain::parser::parse_repository;
use autocontain::db::{initialize_db, insert_repository};
use autocontain::models::Repository;
use rusqlite::Connection;

// TODO: Comment everything for better transparency

fn main() {

    let conn = Connection::open("autocontain.db").expect("Failed to connect to database.");
    initialize_db(&conn).expect("Failed to initialize database.");

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
        // For test parsing
        // TODO: Move to process_repository
        // TODO: After moving, rewrite import
        "parse" => {
            if args.len() < 3 {
                eprintln!("Please provide the path to the repository for parsing.");
                print_usage_and_exit();
            }
            let repo_name = &args[2];
            let repo_path = format!("source/{}", repo_name);
            // For testing parsing
            // let repo_name = "test";
            // let repo_path = "test";
            let repo = Repository {
                id: None,
                name: repo_name.to_string(),
                description: None
            };
            let repo_id = insert_repository(&conn, &repo).expect("Failed to insert repository");
            parse_repository(&repo_path, &conn, repo_id);
            println!("Parsing completed successfully for repository at {}", repo_path);
        }
        _ => {
            eprintln!("Invalid argument '{}'", args[1]);
            print_usage_and_exit();
        }
    }
}
