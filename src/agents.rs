use reqwest::blocking::Client;
use serde_json::json;
use std::error::Error;
use std::io;
use std::collections::HashMap;
use std::path::PathBuf;

// Agent 1: Documentation Analysis Agent
pub fn documentation_analysis_agent(content: &str, openai_api_key: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let prompt = format!(
        "Please analyze the following repository documentation content and provide a response in JSON format. \
        The JSON should contain the following attributes:\n\n\
        1. \"functionalities\": Describe the main functionalities or purpose of the repository.\n\
        2. \"prerequisites\": List any prerequisites needed to use this repository.\n\
        3. \"requirements\": List the software dependencies required to run the repository.\n\
        4. \"installation\": Provide the installation steps in sequence.\n\
        If you cannot find information for any of the fields, respond with an empty string for that field.\n\n\
        ---\n\n{}",
        content
    );

    let response = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": "You are an assistant that helps summarize repository documentation in JSON format."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.5,
            "max_tokens": 500
        }))
        .send()?
        .json::<serde_json::Value>()?;

    // Check if the response contains an error
    if let Some(error) = response.get("error") {
        println!("OpenAI API Error: {}", error["message"].as_str().unwrap_or("Unknown error"));
        return Err(Box::new(io::Error::new(
            io::ErrorKind::Other,
            "OpenAI API returned an error",
        )));
    }

    Ok(response["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
}

// Agent 2: Docker File Generation Agent (only if Docker files are not found)
pub fn docker_file_generation_agent(analysis: &str, openai_api_key: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let prompt = format!(
        "Based on the following analysis of repository requirements, prerequisites, and installation steps, \
        generate only the Dockerfile content. Provide the content as raw text, without any explanations, \
        introductory text, or formatting markers (such as ```Dockerfile or any other symbols).\n\n---\n\n{}",
        analysis
    );

    let response = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": "You are an assistant that generates Docker configuration files based on repository requirements."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.5,
            "max_tokens": 300
        }))
        .send()?
        .json::<serde_json::Value>()?;

    Ok(response["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
}

// TODO: something is wrong with this function
// Agent 3: Run Script Generation Agent
pub fn run_script_generation_agent(
    docker_content: &HashMap<String, String>, 
    openai_api_key: &str,
    dockerfile_path: &str,          // Path to the Dockerfile
    repo_path: PathBuf,                // Path to the repository
    docker_compose_path: Option<&str> // Optional path to the Docker Compose file
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    
    // Check if both Dockerfile and Docker Compose file are present
    let has_dockerfile = docker_content.contains_key("Dockerfile");
    let has_compose_file = docker_compose_path.is_some();
    
    let repo_path_str = repo_path.to_string_lossy().to_string();

    // Create the prompt based on available files
    let prompt = if has_dockerfile && has_compose_file {
        format!(
            "Given the following Docker-related files in the repository, generate a shell script to set up and run the application. \
            The Dockerfile is located at '{}', and the Docker Compose file is located at '{}'. Use 'docker-compose up' if available \
            to manage multi-container setups and service orchestration. If only a Dockerfile is available, use 'docker build' with \
            the Dockerfile path and 'docker run' with the built image. Provide the content as raw text, without any explanations, \
            introductory text, or formatting markers (such as ```Dockerfile or any other symbols).\n\n\
            Repository path: {}\nDockerfile path: {}\nDocker Compose path: {}\n\nDockerfile:\n{}\n\nCompose File:\n{} \
            The repository is cloned under the 'source' folder, and the docker-related scripts are located under the 'scripts' folder, and these 2 folders are on a same hierchy.",
            dockerfile_path,
            docker_compose_path.unwrap_or(""),
            &repo_path_str,
            dockerfile_path,
            docker_compose_path.unwrap_or(""),
            docker_content.get("Dockerfile").unwrap_or(&String::new()),
            docker_content.get("docker-compose.yml").unwrap_or(&String::new())
        )
    } else if has_dockerfile {
        format!(
            "Given only a Dockerfile located at '{}', generate a shell script to build and run the Docker container using \
            'docker build' and 'docker run' with paths specified. Ensure the script is practical for a typical application \
            setup. Provide the content as raw text, without any explanations, introductory text, or formatting markers \
            (such as ```Dockerfile or any other symbols).\n\nRepository path: {}\nDockerfile path: {}\n\nDockerfile:\n{} \
            The repository is cloned under the 'source' folder, and the docker-related scripts are located under the 'scripts' folder, and these 2 folders are on a same hierchy.",
            dockerfile_path,
            &repo_path_str,
            dockerfile_path,
            docker_content.get("Dockerfile").unwrap()
        )
    } else {
        return Err("No Docker-related files found to generate a run script.".into());
    };

    let response = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": "You are an assistant that generates scripts to run Docker configurations."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.5,
            "max_tokens": 300
        }))
        .send()?
        .json::<serde_json::Value>()?;

    let answer = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to retrieve response text from OpenAI")?;

    Ok(answer.to_string())
}