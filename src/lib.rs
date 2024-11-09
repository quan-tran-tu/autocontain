pub mod agents;
pub mod parser;
pub mod db;
pub mod models;
pub mod repo;
pub mod utils;

use std::error::Error;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

use rusqlite::Connection;

use agents::{documentation_analysis_agent, docker_file_generation_agent, run_script_generation_agent};
use repo::{check_github_repo, clone_repo, cleanup_repos, find_and_merge_content, apply_tag, view_basic_analysis, view_tree_structure, install_repo, chat_with_assistant, parse_repo};
use db::initialize_db;

fn agents_caller(
    local_path: PathBuf, // Repository's path on machine
    md_content: String, // Markdown content
    docker_content: &mut HashMap<String, String>, // Docker-related content
    openai_api_key: &str,
    scripts_path: PathBuf, // Path to store repo analysis result and installation script returned from OpenAI API
) -> bool {
    // Merge all docker contents into 1 string
    let docker_combined = docker_content.values().cloned().collect::<Vec<String>>().join("\n\n");
    // Merge docker content and markdown content into 1 string
    let combined_content = format!("Markdown content:\n{}\n\nDocker content:\n{}", md_content, docker_combined);

    // Call the analysis agent to give a basic view about the repository
    let result = documentation_analysis_agent(&combined_content, openai_api_key).and_then(|analysis| {
        // When received result from the agent
        // Write to analysis.md
        fs::write(scripts_path.join("analysis.md"), &analysis)?;
        
        // Call another agent to generate a Dockerfile if no docker-related contents is found
        if docker_content.is_empty() {
            println!("No Docker-related files found. Generating Dockerfile.");
            let generated_dockerfile = docker_file_generation_agent(&analysis, openai_api_key)?;
            fs::write(local_path.join("Dockerfile"), &generated_dockerfile)?;
            docker_content.insert("Dockerfile".to_string(), generated_dockerfile);
        }
        // Currently assume the name of the Dockerfile is 'Dockerfile'
        let dockerfile_path = local_path.join("Dockerfile");
        let dockerfile_path_str = dockerfile_path.to_str().unwrap();
        // Get docker_compose path (if there is any) from the docker_content HashMap
        let docker_compose_path = docker_content.keys()
            .find(|key| key.ends_with(".yml") || key.ends_with(".yaml"))
            .map(|key| local_path.join(key));
        let docker_compose_path_str = docker_compose_path.as_deref().and_then(|p| p.to_str());
        // Call another agent to generate the run script to install the container from docker-related file
        let run_script = run_script_generation_agent(&docker_content, openai_api_key, dockerfile_path_str, docker_compose_path_str)?;
        fs::write(scripts_path.join("run.sh"), run_script)?;

        Ok::<(), Box<dyn Error>>(())
    });

    if let Err(e) = result {
        eprintln!("Error in calling agents: {}", e);
        return false;
    }
    
    true
}

pub fn process_repository(link: &str, openai_api_key: &str, persist: bool, depth: usize) -> Result<(String, PathBuf, PathBuf), Box<dyn Error>> {
    // Check if the GitHub repository exists
    if !check_github_repo(link)? {
        eprintln!("Repository link is invalid or inaccessible.");
        return Err("Repository link is invalid or inaccessible.".into());
    }

    // Clone the repository (or skip if already cloned)
    let (repo_name, local_path) = clone_repo(link, persist)?;

    // Initialize and connect to the database
    let conn = Connection::open("autocontain.db").expect("Failed to connect to database.");
    initialize_db(&conn).expect("Failed to initialize database.");

    // Parsing the repo to the database
    parse_repo(&repo_name, &local_path.to_string_lossy().to_string(), conn);

    // Generating scripts part
    let scripts_path = Path::new("scripts").join(repo_name.clone());
    if !scripts_path.exists() {
        fs::create_dir_all(&scripts_path)?;
        // Analyze documentation and Docker-related files
        let (md_content, _, mut docker_content) = find_and_merge_content(&local_path, depth)?;
        
        // Call the agents
        if agents_caller(local_path.clone(), md_content, &mut docker_content, &openai_api_key, scripts_path.clone()) {
            println!("Repository processed successfully, files saved in '{}'.", scripts_path.display());
        } else {
            println!("Repository processed, failed to call OpenAI.");
        }
    } else {
        println!("Scripts already exists. Not calling agents.")
    }

    // Apply tag if --persist is specified
    if persist {
        apply_tag(&repo_name);
    }

    Ok((repo_name, local_path, scripts_path))
}

pub fn run_menu(persist: bool, local_path: &Path, scripts_path: &Path) {
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
            "1" => view_basic_analysis(scripts_path),
            "2" => view_tree_structure(local_path),
            "3" => install_repo(scripts_path),
            "4" => chat_with_assistant(),
            _ => println!("Invalid choice, please try again."),
        }

        println!(); // Print a newline for better readability
    }
}