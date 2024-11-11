use std::error::Error;
use std::collections::HashMap;

use serde_json::json;

use crate::utils::send_openai_request;
use crate::config::OPENAI_MODEL_NAME;

// Agent 1: Documentation Analysis Agent
pub fn documentation_analysis_agent(content: &str) -> Result<String, Box<dyn Error>> {
    let prompt = format!(
        "Please analyze the following repository documentation content and provide a response in Markdown format
        without formatting markers (such as ```markdown or any other symbols). \
        The Markdown should contain the following sections:\n\n\
        ## Functionalities\nDescribe the main functionalities or purpose of the repository.\n\n\
        ## Prerequisites\nList any prerequisites needed to use this repository.\n\n\
        ## Requirements\nList the software dependencies required to run the repository.\n\n\
        ## Installation\nProvide the installation steps in sequence.\n\n\
        If you cannot find information for any of the sections, respond with an empty entry for that section.\n\n\
        ---\n\n{}",
        content
    );
    let messages = [
        json!({"role": "system", "content": "You are an assistant that helps summarize repository documentation in Markdown format."}),
        json!({"role": "user", "content": prompt}),
    ];
    send_openai_request(OPENAI_MODEL_NAME, &messages, 0.5, 500)
}

// Agent 2: Docker File Generation Agent (only if Docker files are not found)
pub fn docker_file_generation_agent(analysis: &str) -> Result<String, Box<dyn Error>> {
    let prompt = format!(
        "Based on the following analysis of repository requirements, prerequisites, and installation steps, \
        generate only the Dockerfile content. Provide the content as raw text, without any explanations, \
        introductory text, or formatting markers (such as ```Dockerfile or any other symbols).\n\n---\n\n{}",
        analysis
    );
    let messages = [
        json!({"role": "system", "content": "You are an assistant that generates Docker configuration files based on repository requirements."}),
        json!({"role": "user", "content": prompt}),
    ];
    send_openai_request(OPENAI_MODEL_NAME, &messages, 0.5, 300)
}

// TODO: Add check if the container is already installed, or container name has been used.
// Agent 3: Run Script Generation Agent
pub fn run_script_generation_agent(
    docker_content: &HashMap<String, String>, 
    dockerfile_path: &str,     
    docker_compose_path: Option<&str> 
) -> Result<String, Box<dyn Error>> {
    // Check if Docker Compose file or Dockerfile is present
    let has_compose_file = docker_compose_path.is_some();
    let has_dockerfile = docker_content.contains_key("Dockerfile");

    // Create the prompt based on available files
    let prompt = if has_compose_file {
        // Use Docker Compose if available
        format!(
            "Generate a shell script to set up and run the application using Docker Compose with the specific path provided. \
            Use:\n  `docker compose -f {}`\n\n\
            Do not include any 'cd' commands to change directories. Provide the script content as raw text without any introductory text, \
            formatting markers, or explanations.\n\n\
            Docker Compose path: {}\n\nCompose File:\n{}",
            docker_compose_path.unwrap(),
            docker_compose_path.unwrap(),
            docker_content.get("docker-compose.yml").unwrap_or(&String::new())
        )
    } else if has_dockerfile {
        // Fallback to Dockerfile if Docker Compose is not available
        format!(
            "Generate a shell script to build and run the Docker container using the Dockerfile path provided. \
            Use:\n  `docker build -f {}` followed by the appropriate `docker run` command.\n\n\
            Do not include any 'cd' commands to change directories. Provide the script content as raw text without any introductory text, \
            formatting markers, or explanations.\n\n\
            Dockerfile path: {}\n\nDockerfile:\n{}",
            dockerfile_path,
            dockerfile_path,
            docker_content.get("Dockerfile").unwrap()
        )
    } else {
        return Err("No Docker-related files found to generate a run script.".into());
    };
    let messages = [
        json!({"role": "system", "content": "You are an assistant that generates scripts to run Docker configurations."}),
        json!({"role": "user", "content": prompt}),
    ];
    send_openai_request(OPENAI_MODEL_NAME, &messages, 0.5, 300)
}

