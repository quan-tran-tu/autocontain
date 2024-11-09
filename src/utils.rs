use std::path::Path;
use std::io::{self, BufReader, BufRead};
use std::process::{self, Command, Stdio};
use std::fs;

// Print out the program usage then exit
pub fn print_usage_and_exit() {
    eprintln!("Usage:");
    eprintln!(" cargo run -- <github_repo_link> [--persist] [--depth]");
    eprintln!(" cargo run -- rm <repo_link_or_name>");
    process::exit(1);
}

// TODO: Fix install_repo and run_script and script generating prompt in agent
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
            Ok(status) => eprintln!("Command exited with status: {}", status),
            Err(e) => eprintln!("Failed to execute command: {}", e),
        }
    }

    Ok(())
}