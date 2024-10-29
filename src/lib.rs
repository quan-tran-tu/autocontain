mod agents;
pub mod repo;
pub mod utils;

use std::error::Error;
use std::path::Path;

use agents::{documentation_analysis_agent, docker_file_generation_agent, run_script_generation_agent};
use repo::{check_github_repo, clone_repo, create_scripts_folder, find_and_merge_content, copy_docker_files, apply_tag};
use std::fs;

pub fn process_repository(link: &str, openai_api_key: &str, persist: bool) -> Result<(), Box<dyn Error>> {
    // Step 1: Check if the GitHub repository exists
    if !check_github_repo(link)? {
        eprintln!("Repository link is invalid or inaccessible.");
        return Ok(());
    }

    // Step 2: Clone the repository (or skip if already cloned)
    clone_repo(link, persist)?;

    // Step 3: Determine the local path of the cloned repository
    let repo_name = link.trim_end_matches('/').split('/').last().unwrap();
    let local_path = Path::new("source").join(repo_name);

    // Step 4: Analyze documentation and Docker-related files
    let (md_content, md_file_count, docker_content) = find_and_merge_content(&local_path)?;
    println!("Found {} Markdown (.md) files.", md_file_count);

    // Step 5: Prepare combined content for OpenAI analysis
    let docker_combined = docker_content.values().cloned().collect::<Vec<String>>().join("\n\n");
    let combined_content = format!("Markdown content:\n{}\n\nDocker content:\n{}", md_content, docker_combined);

    // Step 6: Analyze documentation with OpenAI for general insights
    let analysis = documentation_analysis_agent(&combined_content, openai_api_key)?;

    // Step 7: Create unique scripts folder for this repository
    let scripts_path = create_scripts_folder(repo_name)?;

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

    println!("Repository processed successfully, files saved in '{}'.", scripts_path.display());

    // Step 10: Apply tag if --persist is specified
    if persist {
        apply_tag(repo_name);
    }

    Ok(())
}
