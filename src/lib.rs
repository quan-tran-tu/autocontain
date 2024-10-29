
use serde_json::json;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use git2::Repository;
use std::path::Path;
use std::fs::{self, OpenOptions};
use std::io::{self, Write, BufRead};
use std::collections::HashSet;
use std::error::Error;

// Function to check if the GitHub repository exists by sending an HTTP request
pub fn check_github_repo(link: &str) -> Result<bool, reqwest::Error> {
    let res = reqwest::blocking::get(link)?;
    Ok(res.status() != StatusCode::NOT_FOUND)
}

pub fn clone_repo(link: &str, persist: bool) -> Result<(), git2::Error> {
    let base_path = Path::new("source");
    if !base_path.exists() {
        fs::create_dir(base_path).expect("Failed to create 'source' folder");
    }

    let repo_name = link.trim_end_matches('/').split('/').last().unwrap();
    let local_path = base_path.join(repo_name);

    // Load tags once and pass it to add_tag/remove_tag functions
    let mut tags = load_tags();

    // Log the current tags to confirm loading
    println!("Current tags: {:?}", tags);

    // Determine if the repository needs cloning
    if local_path.exists() {
        println!("Repository '{}' already exists; skipping clone.", repo_name);
    } else {
        // Remove existing folder if re-cloning is needed
        if local_path.exists() {
            fs::remove_dir_all(&local_path).expect("Failed to remove existing repository");
        }

        println!("Cloning repository into: {:?}", local_path.display());
        Repository::clone(link, &local_path)?;
        println!("Repository successfully cloned.");
    }

    // Update the tags based on the persist flag
    if persist {
        println!("Persist flag is set, adding tag for '{}'", repo_name);
        add_tag(repo_name, &mut tags);
    } else {
        println!("Persist flag is not set, removing tag for '{}'", repo_name);
        remove_tag(repo_name, &mut tags);
    }

    // Save tags back to the file after modifications
    save_tags(&tags);
    println!("Updated tags: {:?}", tags);

    Ok(())
}


// Adds a repository name to the tags HashSet
fn add_tag(repo_name: &str, tags: &mut HashSet<String>) {
    tags.insert(repo_name.to_string());
}

// Removes a repository name from the tags HashSet
fn remove_tag(repo_name: &str, tags: &mut HashSet<String>) {
    tags.remove(repo_name); 
}

// Loads the tags from tags.txt into a HashSet
fn load_tags() -> HashSet<String> {
    let path = Path::new("tags.txt");
    if !path.exists() {
        return HashSet::new();
    }

    let file = fs::File::open(path).expect("Failed to open tags.txt.");
    let reader = io::BufReader::new(file);

    reader
        .lines()
        .filter_map(|line| line.ok())
        .collect()
}

// Saves the current tags HashSet to tags.txt, overwriting any existing contents
fn save_tags(tags: &HashSet<String>) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("tags.txt")
        .expect("Failed to open tags.txt");

    for tag in tags {
        writeln!(file, "{}", tag).expect("Failed to write to tags.txt.");
    }
}

// Cleans up the source directory by deleting any repository folder not in tags.txt
pub fn cleanup_repos() {
    let tags = load_tags();
    let base_path = Path::new("source");

    if base_path.exists() {
        for entry in fs::read_dir(base_path).expect("Failed to read 'source' directory") {
            if let Ok(entry) = entry {
                if let Ok(repo_name) = entry.file_name().into_string() {
                    if !tags.contains(&repo_name) {
                        println!("Removing repository: {}", repo_name);
                        fs::remove_dir_all(entry.path()).expect("Failed to remove repository.");
                    }
                } else {
                    eprintln!("Warning: Skipping non-UTF-8 filename in 'source' directory.");
                }
            }
        }
    }
}

// Analyzes all Markdown (.md) files in a directory and queries OpenAI for summary
pub fn analyze_markdown(dir: &Path, openai_api_key: &str) -> Result<String, Box<dyn Error>> {
    let (merged_content, md_file_count) = find_and_merge_markdown(dir)?;
    println!("Found {} Markdown (.md) files.", md_file_count); // Log the count of .md files

    let analysis = query_openai(&merged_content, openai_api_key)?;
    Ok(analysis)
}

// Recursively finds and merges the content of all Markdown files in a directory
fn find_and_merge_markdown(dir: &Path) -> Result<(String, usize), io::Error> {
    let mut merged_content = String::new();
    let mut md_file_count = 0;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let (content, count) = find_and_merge_markdown(&path)?;
            merged_content.push_str(&content);
            md_file_count += count;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            md_file_count += 1;
            let content = fs::read_to_string(&path)?;
            merged_content.push_str(&content);
            merged_content.push_str("\n\n");
        }
    }
    Ok((merged_content, md_file_count))
}

// Queries the OpenAI API with the provided content to get the main idea, prerequisites, and installation steps
fn query_openai(content: &str, openai_api_key: &str) -> Result<String, Box<dyn Error>> {
    let client = Client::new();
    let prompt = format!(
        "Analyze the following repository documentation content. Answer the following questions as accurately as possible. \
        If you cannot find information for any question, respond with 'Not found' for that question.\n\n\
        1. What is the main idea of the repository?\n\
        2. What are the prerequisites and requirements?\n\
        3. What are the installation steps?\n\n\
        ---\n\n{}",
        content
    );

    let response = client.post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", openai_api_key))
        .json(&json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {"role": "system", "content": "You are an assistant that helps summarize repository documentation."},
                {"role": "user", "content": prompt}
            ],
            "temperature": 0.5,
            "max_tokens": 300
        }))
        .send()?
        .json::<serde_json::Value>()?;

    // Print the entire response for debugging purposes
    println!("Full OpenAI API Response: {:#?}", response);

    // Extract and return the response text
    let answer = response["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to retrieve response text from OpenAI")?;
        
    Ok(answer.to_string())
}
