use std::path::Path;
use std::io::{self, BufReader, BufRead};
use std::process::{self, Command, Stdio};
use std::error::Error;
use std::fs;

use serde_json::{self, json};
use reqwest::blocking::Client;

use crate::config::OPENAI_API_KEY;

// Print out the program usage then exit
pub fn print_usage_and_exit() {
    eprintln!("Usage:");
    eprintln!(" cargo run -- <github_repo_link> [--persist] [--depth]");
    eprintln!(" cargo run -- rm <repo_link_or_name>");
    process::exit(1);
}

// Execute run.sh to install docker container
pub fn run_script(script_path: &Path) -> io::Result<()> {
    let file = fs::File::open(script_path)?;
    let reader = BufReader::new(file);

    // Execute each line in the shell
    for line in reader.lines() {
        let command = line?;
        
        // Skip empty lines and comments
        if command.trim().is_empty() || command.trim().starts_with('#') {
            continue;
        }

        println!("Executing command: {}", &command);

        // Currently only on Windows
        #[cfg(target_os = "windows")]
        let status = Command::new("cmd")
            .arg("/C")
            .arg(&command)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();

        match status {
            Ok(status) if status.success() => continue,
            Ok(status) => {
                eprintln!("Command exited with status: {}", status);
                return Err(io::Error::new(io::ErrorKind::Other, "Docker commands failed"));
            }
            Err(e) => {
                eprintln!("Failed to execute command: {}", e);
                return Err(e);
            }
        }
    }

    Ok(())
}

// OpenAI request function general format
pub fn send_openai_request(
    model_name: &str,
    messages: &[serde_json::Value],
    temperature: f64,
    max_tokens: u32,
) -> Result<String, Box<dyn Error>> {
    let client = Client::new();

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", OPENAI_API_KEY.to_string()))
        .json(&json!({
            "model": model_name,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens
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

    // Extract and return the assistant's response content
    Ok(response["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string())
}