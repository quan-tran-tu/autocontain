use std::error::Error;
use std::io;
use std::collections::HashMap;

use reqwest::blocking::Client;
use serde_json::json;

const OPENAI_MODEL_NAME: &str = "gpt-4o-mini";

// Agent 1: Documentation Analysis Agent
pub fn documentation_analysis_agent(content: &str, openai_api_key: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
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

    let response = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .json(&json!({
            "model": &OPENAI_MODEL_NAME,
            "messages": [
                {"role": "system", "content": "You are an assistant that helps summarize repository documentation in Markdown format."},
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
            "model": &OPENAI_MODEL_NAME,
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

// TODO: Add check if the container is already installed, or container name has been used.
// Agent 3: Run Script Generation Agent
pub fn run_script_generation_agent(
    docker_content: &HashMap<String, String>, 
    openai_api_key: &str,
    dockerfile_path: &str,     
    docker_compose_path: Option<&str> 
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    
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

    let response = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .json(&json!({
            "model": &OPENAI_MODEL_NAME,
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

