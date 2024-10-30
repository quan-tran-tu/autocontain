use reqwest::StatusCode;
use git2::Repository;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write, Read};
use std::path::{Path, PathBuf};

// Function to check if the GitHub repository exists by sending an HTTP request
pub fn check_github_repo(link: &str) -> Result<bool, reqwest::Error> {
    let res = reqwest::blocking::get(link)?;
    Ok(res.status() != StatusCode::NOT_FOUND)
}

// Clones the GitHub repository to the 'source' directory and manages tagging based on the persist flag
pub fn clone_repo(link: &str, persist: bool) -> Result<(String, PathBuf), git2::Error> {
    let base_path = Path::new("source");
    if !base_path.exists() {
        fs::create_dir(base_path).expect("Failed to create 'source' folder");
    }

    let repo_name = link.trim_end_matches('/').split('/').last().unwrap().to_string();
    let local_path = base_path.join(&repo_name);

    // Load tags once and pass it to add_tag/remove_tag functions
    let mut tags = load_tags();

    println!("Current tags: {:?}", tags);

    // Clone if the repository does not exist locally
    if !local_path.exists() {
        println!("Cloning repository into: {:?}", local_path.display());
        Repository::clone(link, &local_path)?;
        println!("Repository successfully cloned.");
    } else {
        println!("Repository '{}' already exists; skipping clone.", repo_name);
    }

    // Update tags based on the persist flag
    if persist {
        println!("Persist flag is set, adding tag for '{}'", repo_name);
        add_tag(&repo_name, &mut tags);
    } else {
        println!("Persist flag is not set, removing tag for '{}'", repo_name);
        remove_tag(&repo_name, &mut tags);
    }

    // Save tags to the file after modifications
    save_tags(&tags);
    println!("Updated tags: {:?}", tags);

    // Return `repo_name` and `local_path` along with `Ok`
    Ok((repo_name, local_path))
}
// Adds a repository name to the tags HashSet
fn add_tag(repo_name: &str, tags: &mut HashSet<String>) {
    tags.insert(repo_name.to_string());
}

// Removes a repository name from the tags HashSet
fn remove_tag(repo_name: &str, tags: &mut HashSet<String>) {
    tags.remove(repo_name);
}

// Applies a tag to the specified repository
pub fn apply_tag(repo_name: &str) {
    let mut tags = load_tags();
    add_tag(repo_name, &mut tags);
    save_tags(&tags);
}

// Copies Docker-related files to the unique 'scripts/{repo_name}' folder
pub fn copy_docker_files(docker_content: &HashMap<String, String>, scripts_path: &Path) -> io::Result<()> {
    for (file_name, content) in docker_content {
        let file_path = scripts_path.join(file_name);
        fs::write(&file_path, content)?;
        println!("Copied Docker-related file to scripts folder: {}", file_name);
    }
    Ok(())
}

// Loads the tags from tags.txt into a HashSet
fn load_tags() -> HashSet<String> {
    let path = Path::new("tags.txt");
    if !path.exists() {
        return HashSet::new();
    }

    let file = fs::File::open(path).expect("Failed to open tags.txt.");
    let reader = io::BufReader::new(file);

    reader.lines().filter_map(|line| line.ok()).collect()
}

// Saves the current tags HashSet to tags.txt, overwriting any existing contents
fn save_tags(tags: &HashSet<String>) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("tags.txt")
        .expect("Failed to open tags.txt.");

    for tag in tags {
        writeln!(file, "{}", tag).expect("Failed to write to tags.txt.");
    }
}

// Cleans up the unique 'scripts/{repo_name}' directory if it's not tagged
pub fn cleanup_repos() {
    let tags = load_tags();
    
    // Clean up the 'scripts' folder
    let scripts_base_path = Path::new("scripts");
    if scripts_base_path.exists() {
        for entry in fs::read_dir(scripts_base_path).expect("Failed to read 'scripts' directory") {
            if let Ok(entry) = entry {
                if let Ok(repo_name) = entry.file_name().into_string() {
                    if !tags.contains(&repo_name) {
                        println!("Removing scripts folder for repository: {}", repo_name);
                        fs::remove_dir_all(entry.path()).expect("Failed to remove scripts folder.");
                    }
                } else {
                    eprintln!("Warning: Skipping non-UTF-8 filename in 'scripts' directory.");
                }
            }
        }
    }

    // Clean up the 'source' folder
    let source_base_path = Path::new("source");
    if source_base_path.exists() {
        for entry in fs::read_dir(source_base_path).expect("Failed to read 'source' directory") {
            if let Ok(entry) = entry {
                if let Ok(repo_name) = entry.file_name().into_string() {
                    if !tags.contains(&repo_name) {
                        println!("Removing source folder for repository: {}", repo_name);
                        fs::remove_dir_all(entry.path()).expect("Failed to remove source folder.");
                    }
                } else {
                    eprintln!("Warning: Skipping non-UTF-8 filename in 'source' directory.");
                }
            }
        }
    }
}

// TODO: add --depth {int} to search for .md files
// Scans the repository directory to find Markdown and Docker-related files, and returns their content.
// - Markdown content is concatenated and returned as a single string.
// - Docker-related files are stored in a HashMap with their filename as the key.
pub fn find_and_merge_content(dir: &Path) -> Result<(String, usize, HashMap<String, String>), io::Error> {
    let mut md_content = String::new();
    let mut md_file_count = 0;
    let mut docker_content = HashMap::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let (nested_md_content, count, nested_docker_content) = find_and_merge_content(&path)?;
            md_content.push_str(&nested_md_content);
            md_file_count += count;
            
            // Merge nested Docker content into the main hashmap
            for (file_name, content) in nested_docker_content {
                docker_content.insert(file_name, content);
            }
        } else {
            let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or_default().to_string();

            if file_name.ends_with(".md") {
                // Recognize Markdown files
                md_file_count += 1;
                let content = fs::read_to_string(&path)?;
                md_content.push_str(&content);
                md_content.push_str("\n\n");
            } else if file_name == "Dockerfile" || file_name.ends_with(".yml") || file_name.ends_with(".yaml") {
                // Recognize Docker-related files by filename or inspection for Docker Compose
                let content = fs::read_to_string(&path)?;

                if file_name == "Dockerfile" || content.contains("services") {
                    docker_content.insert(file_name, content);
                }
            }
        }
    }

    Ok((md_content, md_file_count, docker_content))
}


// Placeholder functions for each option
pub fn view_basic_analysis(scripts_path: &Path) {
    let analysis_path = scripts_path.join("analysis.md");
    println!("Viewing repository's basic analysis...");
    // Check if the file exists
    if !analysis_path.exists() {
        println!("No analysis.md file found at {}", scripts_path.display());
        return;
    }

    // Open and read the file content
    match fs::File::open(&analysis_path) {
        Ok(mut file) => {
            let mut content = String::new();
            if file.read_to_string(&mut content).is_ok() {
                println!("Content of analysis.md:\n{}", content);
            } else {
                println!("Failed to read the content of analysis.md");
            }
        },
        Err(err) => {
            println!("Failed to open analysis.md: {}", err);
        }
    }
}

pub fn view_tree_structure(local_path: &Path) {
    println!("Displaying repository's tree structure...");
    // Add logic to show the repo's tree structure
    display_tree_structure(local_path, 0, "");
}
fn display_tree_structure(path: &Path, level: usize, prefix: &str) {
    if let Ok(entries) = fs::read_dir(path) {
        let entries: Vec<_> = entries.filter_map(Result::ok).collect();
        let count = entries.len();

        for (i, entry) in entries.into_iter().enumerate() {
            let entry_path = entry.path();
            let file_name = entry.file_name().into_string().unwrap_or_default();
            let is_last = i == count - 1;

            // Printing the current line with branch symbols
            println!("{}{}─ {}", prefix, if is_last { "└" } else { "├" }, file_name);

            // Prepare the new prefix for the next level
            let new_prefix = format!("{}{}", prefix, if is_last { "  " } else { "│ " });

            // Recursively display tree structure for directories
            if entry_path.is_dir() {
                display_tree_structure(&entry_path, level + 1, &new_prefix);
            }
        }
    } else {
        println!("Failed to read the directory: {:?}", path);
    }
}

pub fn install_repo() {
    println!("Installing repository...");
    // TODO: Execute script to install container
    // TODO: Log out the output from the container
}

pub fn chat_with_assistant() {
    println!("Starting chat with assistant...");
    // TODO: Add database to store informations about the code
    // TODO: Build RAG system
}
