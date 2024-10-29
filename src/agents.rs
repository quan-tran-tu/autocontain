use reqwest::blocking::Client;
use serde_json::json;
use std::error::Error;
use std::collections::HashMap;

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

    Ok(response["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
}

// Agent 2: Docker File Generation Agent (only if Docker files are not found)
pub fn docker_file_generation_agent(analysis: &str, openai_api_key: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let prompt = format!(
        "Based on the following analysis of repository requirements, prerequisites, and installation steps, \
        generate a Dockerfile or docker-compose.yml script to install and run the repository.\n\n---\n\n{}",
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

// Agent 3: Run Script Generation Agent
pub fn run_script_generation_agent(
    docker_content: &HashMap<String, String>, 
    openai_api_key: &str
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    
    // Check if both Dockerfile and Docker Compose file are present
    let has_dockerfile = docker_content.contains_key("Dockerfile");
    let has_compose_file = docker_content.keys().any(|k| k.ends_with("yml") || k.ends_with("yaml"));
    
    // Create the prompt based on available files
    let prompt = if has_dockerfile && has_compose_file {
        format!(
            "Given the following Docker-related files, generate a shell script to set up and run the application using \
            the most appropriate commands. Prioritize using docker-compose if it is available, as it will handle multi-container \
            setups and service orchestration. If docker-compose.yml or equivalent is available, use 'docker-compose up'. Otherwise, \
            if only a Dockerfile is present, use 'docker build' and 'docker run'.\n\n\
            Dockerfile:\n{}\n\nCompose File:\n{}",
            docker_content.get("Dockerfile").unwrap_or(&String::new()),
            docker_content.get("docker-compose.yml").or(docker_content.get("compose.yml")).or(docker_content.get("program.yml")).unwrap_or(&String::new())
        )
    } else if has_dockerfile {
        format!(
            "Given only a Dockerfile, generate a shell script to build and run the Docker container using 'docker build' and 'docker run'. \
            Ensure the script is practical for a typical application setup.\n\n\
            Dockerfile:\n{}",
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
