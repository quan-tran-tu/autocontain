mod agents;
pub mod repo;
pub mod utils;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

use agents::{documentation_analysis_agent, docker_file_generation_agent, run_script_generation_agent};
use repo::{check_github_repo, clone_repo, cleanup_repos, find_and_merge_content, copy_docker_files, apply_tag, view_basic_analysis, view_tree_structure, install_repo, chat_with_assistant};


fn agents_caller(md_content: String, docker_content: HashMap<String, String>, openai_api_key: &str, scripts_path: PathBuf) -> Result<(), Box<dyn Error>> {
    // Step 5: Prepare combined content for OpenAI analysis
    let docker_combined = docker_content.values().cloned().collect::<Vec<String>>().join("\n\n");
    let combined_content = format!("Markdown content:\n{}\n\nDocker content:\n{}", md_content, docker_combined);
    // Step 6: Analyze documentation with OpenAI for general insights
    let analysis = documentation_analysis_agent(&combined_content, openai_api_key)?;
    // TODO: Create a md file and write analysis into it
    if docker_content.is_empty() {
        // Step 8a: If no Docker files found, generate a Dockerfile with `docker_file_generation_agent`
        println!("No Docker-related files found. Generating Dockerfile.");
        let generated_dockerfile = docker_file_generation_agent(&analysis, openai_api_key)?;
        fs::write(scripts_path.join("Dockerfile"), &generated_dockerfile)?;
    } else {
        // Step 8b: Use `copy_docker_files` to copy all found Docker-related files to the unique scripts folder
        copy_docker_files(&docker_content, &scripts_path)?;
    }
    // Step 9: Generate a run script that considers all Docker-related files
    let run_script = run_script_generation_agent(&docker_content, openai_api_key)?;
    fs::write(scripts_path.join("run_docker.sh"), run_script)?;

    Ok(())
}


pub fn process_repository(link: &str, openai_api_key: &str, persist: bool) -> Result<(), Box<dyn Error>> {
    // Step 1: Check if the GitHub repository exists
    if !check_github_repo(link)? {
        eprintln!("Repository link is invalid or inaccessible.");
        return Ok(());
    }

    // Step 2: Clone the repository (or skip if already cloned)
    let (repo_name, local_path) = clone_repo(link, persist)?;
    let scripts_path = Path::new("scripts").join(repo_name.clone());
    if !scripts_path.exists() {
        fs::create_dir_all(&scripts_path)?;
        // Step 4: Analyze documentation and Docker-related files
        let (md_content, md_file_count, docker_content) = find_and_merge_content(&local_path)?;
        println!("Found {} Markdown (.md) files.", md_file_count);
        agents_caller(md_content, docker_content, &openai_api_key, scripts_path.clone())?;
    } else {
        println!("Scripts already exists. Not calling agents.");
    }

    println!("Repository processed successfully, files saved in '{}'.", scripts_path.display());

    // Step 10: Apply tag if --persist is specified
    if persist {
        apply_tag(&repo_name);
    }

    Ok(())
}

pub fn run_menu(persist: bool) {
    loop {
        // Display the menu
        println!("Choose an option:");
        println!("0. Exit the program.");
        println!("1. View repo's basic analysis.");
        println!("2. View repo's tree structure.");
        println!("3. Install the repo.");
        println!("4. Chat with assistant.");

        // Get user input
        print!("Enter your choice: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            println!("Failed to read line");
            continue;
        }

        let input = input.trim();

        // Match the input to execute corresponding actions
        match input {
            "0" => {
                println!("Exiting program...");
                if !persist {
                    cleanup_repos();
                }
                break;
            },
            "1" => view_basic_analysis(),
            "2" => view_tree_structure(),
            "3" => install_repo(),
            "4" => chat_with_assistant(),
            _ => println!("Invalid choice, please try again."),
        }

        println!(); // Print a newline for better readability
    }
}