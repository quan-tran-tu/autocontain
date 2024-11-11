use std::process;
use std::env;

use autocontain::{process_repository, run_menu};
use autocontain::utils::print_usage_and_exit;
use autocontain::repo::{remove_repo, get_all_repos};

fn main() {
    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage_and_exit();
    }

    match args[1].as_str() {
        "rm" => { // Remove repository from machine, one at a time
            let repo_name = &args[2];
            remove_repo(repo_name);
        }
        "run" => { // Clone the repository, parse the code and generate Docker-related file (if none were found)
            let link = &args[2];
            // Validate GitHub link format
            if !link.starts_with("https://github.com/") {
                eprintln!("Invalid GitHub repository link.");
                process::exit(1);
            }
            // Default values
            let mut persist = false;
            let mut depth = 0;

            // Get tags
            for arg in &args[3..] {
                match arg.as_str() {
                    // Install the repository permanantly
                    "--persist" => persist = true,
                    // How deep the program should search for Markdown files.
                    _ if arg.starts_with("--depth=") => {
                        if let Some(value) = arg.strip_prefix("--depth=") {
                            depth = value.parse::<usize>().unwrap_or(0);
                        }
                    }
                    // Invalid tags for run command
                    _ => {
                        println!("Warning: Invalid argument {}", arg);
                        print_usage_and_exit();
                    },
                }
            }

            // Main function to pre-process the repository
            let (_, local_path, scripts_path, conn) = process_repository(link, persist, depth)
                .expect("Failed to process repository.");
            // Run the cli menu
            run_menu(persist, &local_path, &scripts_path, &conn);
        }
        "list" => { // List all repositories installed
            get_all_repos();
        }
        _ => { // Invalid argument after cargo run --
            eprintln!("Invalid argument '{}'", args[1]);
            print_usage_and_exit();
        }
    }
}
