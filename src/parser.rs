use std::fs;
use std::collections::HashSet;

use rusqlite::Connection;
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;

use crate::models::{Class, Function};
use crate::db::{insert_class, insert_function, insert_dependencies};

// Initializes a tree-sitter parser for Python.
fn initialize_parser() -> Parser {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).expect("Error loading Python grammar");
    parser
}

// Parses a Python repository directory for classes and functions.
pub fn parse_repository(repo_path: &str, conn: &Connection, repo_id: i32) {
    let mut parser = initialize_parser();

    // Walk through each file in the directory and parse Python files
    for entry in WalkDir::new(repo_path) {
        let entry = entry.expect("Failed to access entry");
        if entry.path().extension().map_or(false, |ext| ext == "py") {
            let code = fs::read_to_string(entry.path()).expect("Failed to read file");
            parse_file(&code, &mut parser, conn, repo_id, entry.path().to_str().unwrap());
        }
    }
}

// Parses a single Python file and extracts classes, functions, and their dependencies.
fn parse_file(code: &str, parser: &mut Parser, conn: &Connection, repo_id: i32, file_path: &str) {
    let tree = parser.parse(code, None).expect("Failed to parse code");
    let root_node = tree.root_node();

    extract_classes_and_functions(root_node, code, conn, repo_id, file_path);
}

// Extracts classes and functions information and stores in sqlite database
fn extract_classes_and_functions(root: Node, code: &str, conn: &Connection, repo_id: i32, file_path: &str) {
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "class_definition" => {
                let class = create_class_struct(node, code, repo_id, file_path);
                // Insert the class data into the database
                insert_class(conn, &class).expect("Failed to insert class");
                // Retrieve class_id after insertion to set it for methods
                let class_id = conn.last_insert_rowid() as i32;
                // Process methods of the class and associate them with this class_id
                process_class_methods(node, code, conn, repo_id, class_id, file_path);
            },
            "function_definition" => {
                process_function_definition(node, code, conn, repo_id, None, file_path);
            },
            _ => {}
        }
    }
}

// Helper function to process a function (method) node with kind == "function_definition"
fn process_function_definition(node: Node, code: &str, conn: &Connection, repo_id: i32, class_id: Option<i32>, file_path: &str) {
    let func = create_function_struct(node, code, repo_id, class_id, file_path);
    // Insert the function data into the database
    insert_function(conn, &func).expect("Failed to insert function");
    // Insert function dependencies into the database
    let func_name = func.name.clone();
    let dependencies = extract_dependencies(node, code);
    insert_dependencies(conn, &func_name, &dependencies).expect("Failed to insert dependencies");
}

// Helper function to process methods within a class node
fn process_class_methods(class_node: Node, code: &str, conn: &Connection, repo_id: i32, class_id: i32, file_path: &str) {
    // Recursive function to locate and process method nodes within a class
    fn traverse(node: Node, code: &str, conn: &Connection, repo_id: i32, class_id: i32, file_path: &str) {
        if node.kind() == "function_definition" {
            process_function_definition(node, code, conn, repo_id, Some(class_id), file_path);
        }

        // Recursively visit child nodes
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            traverse(child, code, conn, repo_id, class_id, file_path);
        }
    }
    // Start the traversal from the class body
    traverse(class_node, code, conn, repo_id, class_id, file_path);
}

// Helper function to create a Class struct
fn create_class_struct(node: Node, code: &str, repo_id: i32, file_path: &str) -> Class {
    let class_name = extract_identifier(node, code);
    let docstring = extract_docstring(node, code); // Extract docstring for the class
    let attributes = extract_attributes(node, code);
    let (start_line, end_line) = (node.start_position().row as i32, node.end_position().row as i32);

    Class {
        id: None,
        repo_id,
        name: class_name.unwrap_or("<unknown>".to_string()),
        attributes,
        file_location: file_path.to_string(),
        start_line,
        end_line,
        docstring,
    }
}

// Helper function to create a Function struct with optional class_id
fn create_function_struct(node: Node, code: &str, repo_id: i32, class_id: Option<i32>, file_path: &str) -> Function {
    let func_name = extract_identifier(node, code);
    let parameters = extract_parameters(node, code);
    let return_type = extract_return_type(node, code);
    let docstring = extract_docstring(node, code); // Extract docstring for the function
    let (start_line, end_line) = (node.start_position().row as i32, node.end_position().row as i32);

    Function {
        id: None,
        repo_id,
        class_id,  // Set class_id if it is a method
        name: func_name.unwrap_or("<unknown>".to_string()),
        parameters,
        return_type,
        file_location: file_path.to_string(),
        start_line,
        end_line,
        docstring,
    }
}

// Extracts the identifier (name) for a class or function node.
fn extract_identifier(node: Node, code: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Some(child.utf8_text(code.as_bytes()).unwrap().to_string());
        }
    }
    None
}

// Extracts docstring for a class or function node.
fn extract_docstring(node: Node, code: &str) -> Option<String> {
    // Recursively search for the first `string` node within the node's body
    fn find_docstring(node: Node, code: &str) -> Option<String> {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Check if this child is the `string` node we're looking for
            if child.kind() == "string" {
                return Some(child.utf8_text(code.as_bytes()).unwrap().trim_matches('"').to_string());
            }
            // If the child is a `body` node or another container, continue searching within it
            let docstring = find_docstring(child, code);
            if docstring.is_some() {
                return docstring;
            }
        }
        None
    }

    find_docstring(node, code)
}

// Extracts parameters for a function node or a class methods.
fn extract_parameters(node: Node, code: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameters" {
            let params = child.utf8_text(code.as_bytes()).unwrap();
            return Some(params.trim_matches(|c| c == '(' || c == ')').to_string());
        }
    }
    None
}

// Extracts attributes for a class node.
fn extract_attributes(node: Node, code: &str) -> Option<String> {
    let mut attributes = Vec::new();

    // Recursive function to traverse the class node
    fn traverse(node: Node, code: &str, attributes: &mut Vec<String>) {
        if node.kind() == "function_definition" {
            if let Some(function_name) = extract_identifier(node, code) {
                if function_name == "__init__" {
                    // Extract parameters in __init__ as attributes
                    if let Some(params) = extract_parameters(node, code) {
                        attributes.extend(
                            params
                                .split(',')
                                .skip(1) // Skip "self"
                                .map(|param| {
                                    let param = param.trim();
                                    let parts: Vec<&str> = param.split(':').collect();
                                    let param_name = parts[0].trim();
                                    let param_type = if parts.len() > 1 {
                                        parts[1].trim().trim_end_matches(')').trim()
                                    } else {
                                        "unknown"
                                    };
                                    format!("{}: {}", param_name, param_type)
                                }),
                        );
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            traverse(child, code, attributes);
        }
    }

    // Start traversal from the root node of the class
    traverse(node, code, &mut attributes);

    if !attributes.is_empty() {
        Some(attributes.join(", "))
    } else {
        None
    }
}

// Extracts all other functions called in a function node
fn extract_dependencies(node: Node, code: &str) -> Vec<String> {
    let mut dependencies = HashSet::new();

    // Recursive function to traverse and identify function calls
    fn traverse(node: Node, code: &str, dependencies: &mut HashSet<String>) {
        if node.kind() == "call" {
            if let Some(function_node) = node.child_by_field_name("function") {
                if function_node.kind() == "identifier" {
                    if let Ok(called_function) = function_node.utf8_text(code.as_bytes()) {
                        dependencies.insert(called_function.to_string());
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            traverse(child, code, dependencies);
        }
    }

    traverse(node, code, &mut dependencies);

    dependencies.into_iter().collect()
}

// Extracts return type for a function node.
fn extract_return_type(node: Node, code: &str) -> Option<String> {
    if let Some(return_annotation) = node.child_by_field_name("return_type") {
        return Some(return_annotation.utf8_text(code.as_bytes()).unwrap_or("unknown").to_string());
    }
    None
}
