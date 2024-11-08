use reqwest::StatusCode;
use git2::Repository;
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, Write, Read};
use std::path::{Path, PathBuf};

use crate::utils::run_script;

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

// Scans the repository directory to find Markdown and Docker-related files, and returns their content.
// - Markdown content is concatenated and returned as a single string.
// - Docker-related files are stored in a HashMap.
// - Only 1 Dockerfile and compose file, each, is considered.
pub fn find_and_merge_content(
    dir: &Path,
    depth: usize,
) -> Result<(String, usize, HashMap<String, String>), io::Error> {
    let mut md_content = String::new();
    let mut md_file_count = 0;
    let mut docker_content = HashMap::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if depth > 0 {
                // Recursively call with reduced depth if depth > 0
                let (nested_md_content, count, _) = find_and_merge_content(&path, depth - 1)?;
                md_content.push_str(&nested_md_content);
                md_file_count += count;
            }
        } else {
            let file_name = path.file_name().and_then(|f| f.to_str()).unwrap_or_default().to_string();

            if file_name.ends_with(".md") {
                // Recognize Markdown files based on depth level
                md_file_count += 1;
                let content = fs::read_to_string(&path)?;
                md_content.push_str(&content);
                md_content.push_str("\n\n");
            } else if depth == 0 && (file_name == "Dockerfile" || file_name.ends_with(".yml") || file_name.ends_with(".yaml")) {
                // Collect Docker-related files only at the outermost layer (depth 0)
                let content = fs::read_to_string(&path)?;
                if file_name == "Dockerfile" || content.contains("services") {
                    docker_content.insert(file_name, content);
                }
            }
        }
    }

    Ok((md_content, md_file_count, docker_content))
}

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
    // Exclude specified directories from the tree view
    display_tree_structure(local_path, 0, "");
}

fn display_tree_structure(path: &Path, level: usize, prefix: &str) {
    // Directories to exclude from the tree view
    let excluded_dirs = [
        "node_modules", ".github", ".git", "target", ".idea", ".vscode",
        "__pycache__", "dist", "build", ".DS_Store", ".pytest_cache", "logs",
        "coverage", ".next", "public", "static"
    ];

    if let Ok(entries) = fs::read_dir(path) {
        let entries: Vec<_> = entries.filter_map(Result::ok).collect();
        let count = entries.len();
        
        // Group entries by file extension if they are files
        let mut extension_groups: HashMap<String, Vec<PathBuf>> = HashMap::new();

        for entry in &entries {
            let entry_path = entry.path();
            if entry_path.is_file() {
                let ext = entry_path.extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                extension_groups.entry(ext).or_default().push(entry_path.clone());
            }
        }

        for (i, entry) in entries.into_iter().enumerate() {
            let entry_path = entry.path();
            let file_name = entry.file_name().into_string().unwrap_or_default();
            let is_last = i == count - 1;

            // Skip excluded directories
            if entry_path.is_dir() && excluded_dirs.contains(&file_name.as_str()) {
                continue;
            }

            // Printing the current line with branch symbols
            println!("{}{}─ {}", prefix, if is_last { "└" } else { "├" }, file_name);

            // Prepare the new prefix for the next level
            let new_prefix = format!("{}{}", prefix, if is_last { "  " } else { "│ " });

            // Recursively display tree structure for directories
            if entry_path.is_dir() {
                display_tree_structure(&entry_path, level + 1, &new_prefix);
            } else {
                let ext = entry_path.extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                // Display the first 4 files and replace others with "..." if more than 4
                if let Some(files) = extension_groups.get(&ext) {
                    if files.len() > 4 {
                        for (j, file) in files.iter().take(4).enumerate() {
                            let file_name = file.file_name().unwrap().to_string_lossy();
                            println!("{}{}─ {}", new_prefix, if j == 3 { "└" } else { "├" }, file_name);
                        }
                        println!("{}└─ ...", new_prefix); // Indicating remaining files
                        break;
                    }
                }
            }
        }
    } else {
        println!("Failed to read the directory: {:?}", path);
    }
}
pub fn install_repo(scripts_path: &Path) {
    println!("Installing repository...");
    let script_path = scripts_path.join("run_docker.sh");
    match run_script(&script_path) {
        Ok(_) => println!("Docker container installed."),
        Err(e) => eprintln!("Error installing Docker container: {}.", e),
    }
}

pub fn chat_with_assistant() {
    println!("Starting chat with assistant...");
    // TODO: Write a prompt format to pass to OpenAI API
}

pub fn remove_repo(repo_name: &str) {
    println!("Removing repository '{}'", repo_name);

    // Check if repo_name is in tags.txt
    let tags_path = PathBuf::from("tags.txt");
    let repo_in_tags = if let Ok(file) = fs::File::open(&tags_path) {
        io::BufReader::new(file)
            .lines()
            .filter_map(Result::ok)
            .any(|line| line == repo_name)
    } else {
        eprintln!("Failed to open tags.txt");
        return;
    };

    let source_dir = PathBuf::from("source").join(repo_name);
    let scripts_dir = PathBuf::from("scripts").join(repo_name);

    if repo_in_tags {
        // If repo_name is in tags.txt, try to remove the directories
        if let Err(e) = fs::remove_dir_all(&source_dir) {
            eprintln!("Failed to remove {} in source directory: {}", repo_name, e);
        }
        if let Err(e) = fs::remove_dir_all(&scripts_dir) {
            eprintln!("Failed to remove {} in scripts directory: {}", repo_name, e);
        }
        println!("Repository '{}' removed successfully.", repo_name);

        // Remove repo_name from tags.txt
        if let Ok(file) = fs::File::open(&tags_path) {
            let lines: Vec<String> = io::BufReader::new(file)
                .lines()
                .filter_map(Result::ok)
                .filter(|line| line != repo_name) // Exclude the repo_name
                .collect();

            // Write the filtered lines back to tags.txt
            if let Err(e) = fs::write(&tags_path, lines.join("\n") + "\n") {
                eprintln!("Failed to update tags.txt: {}", e);
            }
        }
    } else if source_dir.exists() {
        // If not in tags.txt but source_dir exists, print a message
        println!("Cannot remove repository '{}' right now.", repo_name);
    } else {
        // If repo_name is neither in tags.txt nor the source directory
        println!("No repository named '{}' installed.", repo_name);
    }
}

pub fn get_all_repos() {
    let source_dir = PathBuf::from("source");

    match fs::read_dir(&source_dir) {
        Ok(entries) => {
            let mut found_repo = false;

            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(repo_name) = path.file_name().and_then(|name| name.to_str()) {
                            println!("- {}", repo_name);
                            found_repo = true;
                        }
                    }
                }
            }
            if !found_repo {
                println!("No repositories installed.");
            }
        }
        Err(e) => eprintln!("Failed to list repositories: {}", e),
    }
}