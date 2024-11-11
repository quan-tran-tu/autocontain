use std::error::Error;
use std::io::{self, Write};

use rusqlite::Connection;
use serde_json::json;

use crate::db::{get_dependencies, get_function_description};
use crate::config::OPENAI_MODEL_NAME;
use crate::utils::send_openai_request;

// Main function to handle continuous chat with the assistant
pub fn chat_with_assistant(conn: &Connection) {
    println!("Starting chat with the assistant. Type '!q' to exit to the main menu.");

    loop {
        // Prompt the user for input
        print!("You: ");
        io::stdout().flush().unwrap();

        let mut user_input = String::new();
        if io::stdin().read_line(&mut user_input).is_err() {
            println!("Failed to read line. Please try again.");
            continue;
        }

        let user_input = user_input.trim();

        // Check for the exit command
        if user_input == "!q" {
            println!("Exiting chat...");
            break;
        }

        // Handle the user's query and print the assistant's response
        match handle_user_query(user_input, conn) {
            Ok(response) => println!("Assistant: {}", response),
            Err(err) => println!("Error: {}", err),
        }

        println!(); // Print a newline for better readability
    }
}

// Function to handle each user query, determining intent and generating a response with OpenAI
pub fn handle_user_query(query: &str, conn: &Connection) -> Result<String, Box<dyn Error>> {
    // Detect User Intent (only "Casual Chat" and "Overall Code Logic")
    let intent = classify_intent(query)?;
    println!("Intent: {}", intent.as_str());

    let content = match intent.as_str() {
        "Overall Code Logic" => {
            // Generate the logic flow for the overall structure of the program
            let logic_flow = format_program_flow(conn)?;
            format!(
                "Provide a summary of the overall code logic for a repository. \
                Here is the code flow:\n\n{}\n\n\
                Summarize the main purpose and flow of the repository.",
                logic_flow
            )
        },
        "Casual Chat" => format!(
            "The user said: '{}'. Respond in a friendly manner.",
            query
        ),
        _ => format!(
            "Unrecognized intent.\n\nUser's Query: '{}'",
            query
        ),
    };
    let messages = [
        json!({"role": "system", "content": "You are an assistant who explains code repository structures, logic flow, and functionality."}),
        json!({"role": "user", "content": content}),
    ];
    send_openai_request(OPENAI_MODEL_NAME ,&messages, 0.5, 1000)
}

// Intent classification function
fn classify_intent(query: &str) -> Result<String, Box<dyn Error>> {
    let prompt = format!(
        "Classify the user query into one of the following categories: \
        ['Casual Chat', 'Overall Code Logic']. \
        Return only the result category. \
        User Query: '{}'", query
    );
    let messages = [
        json!({"role": "system", "content": "You are an assistant that excels in recognizing user's prompt intent."}),
        json!({"role": "user", "content": prompt})
    ];
    send_openai_request(OPENAI_MODEL_NAME, &messages, 0.5, 1000)
}

fn format_program_flow(conn: &Connection) -> Result<String, Box<dyn Error>> {
    // Start with the main function or entry point (assuming "main" is the entry function)
    let mut formatted_flow = String::from("The program follows this logic flow:\n\n");
    let mut visited = std::collections::HashSet::new();

    build_flow(conn, "main", None, &mut formatted_flow, &mut visited, 0)?;
    println!("Format flow: {}", formatted_flow);
    Ok(formatted_flow)
}

// Recursive helper function to build the flow
fn build_flow(
    conn: &Connection,
    function_name: &str,
    class_id: Option<i32>,
    flow: &mut String,
    visited: &mut std::collections::HashSet<(String, Option<i32>)>,
    level: usize,
) -> Result<(), Box<dyn Error>> {
    // Avoid re-processing functions we've already visited
    if !visited.insert((function_name.to_string(), class_id)) {
        return Ok(()); 
    }

    // Fetch function description (docstring)
    let description = get_function_description(conn, function_name, class_id)
        .unwrap_or_else(|_| "No description available".to_string());

    // Indent based on level to show hierarchy
    flow.push_str(&format!(
        "{}- Function: `{}`\n{}  - Purpose: {}\n",
        "  ".repeat(level),
        function_name,
        "  ".repeat(level),
        description
    ));

    // Fetch dependencies for the current function
    let dependencies = get_dependencies(conn, function_name, class_id)?;

    // Recursively add each dependency to the flow
    for (dependency, dep_class_id) in dependencies {
        build_flow(conn, &dependency, dep_class_id, flow, visited, level + 1)?;
    }

    Ok(())
}
