use rusqlite::Connection;
use std::fs;
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;

use crate::models::{Class, Function};
use crate::db::{insert_class, insert_function};

/// Initializes a tree-sitter parser for Python.
fn initialize_parser() -> Parser {
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into()).expect("Error loading Python grammar");
    parser
}

/// Parses a Python repository directory for classes and functions.
pub fn parse_repository(repo_path: &str, conn: &Connection, repo_id: i32) {
    let mut parser = initialize_parser();

    // Walk through each file in the directory and parse Python files
    for entry in WalkDir::new(repo_path) {
        let entry = entry.expect("Failed to access entry");
        if entry.path().extension().map_or(false, |ext| ext == "py") {
            let code = fs::read_to_string(entry.path()).expect("Failed to read file");
            parse_python_file(&code, &mut parser, conn, repo_id, entry.path().to_str().unwrap());
        }
    }
}

/// Parses a single Python file and extracts classes and functions.
fn parse_python_file(code: &str, parser: &mut Parser, conn: &Connection, repo_id: i32, file_path: &str) {
    let tree = parser.parse(code, None).expect("Failed to parse code");
    let root_node = tree.root_node();

    extract_classes_and_functions(root_node, code, conn, repo_id, file_path);
}

/// Extracts classes and functions from a syntax tree and stores them in the database.
fn extract_classes_and_functions(root: Node, code: &str, conn: &Connection, repo_id: i32, file_path: &str) {
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        match node.kind() {
            "class_definition" => {
                let class_name = extract_identifier(node, code);
                let (methods, attributes) = extract_methods_and_attributes(node, code); 
                let (start_line, end_line) = (node.start_position().row as i32, node.end_position().row as i32);

                let class = Class {
                    id: None,
                    repo_id,
                    name: class_name.unwrap_or("<unknown>".to_string()),
                    attributes,
                    methods,
                    file_location: file_path.to_string(),
                    start_line,
                    end_line,
                };
                insert_class(conn, &class).expect("Failed to insert class");
            },
            "function_definition" => {
                let func_name = extract_identifier(node, code);
                let parameters = extract_parameters(node, code);
                let (start_line, end_line) = (node.start_position().row as i32, node.end_position().row as i32);

                let func = Function {
                    id: None,
                    repo_id,
                    name: func_name.unwrap_or("<unknown>".to_string()),
                    parameters,
                    file_location: file_path.to_string(),
                    start_line,
                    end_line,
                };
                insert_function(conn, &func).expect("Failed to insert function");
            },
            _ => {}
        }
    }
}

/// Extracts the identifier (name) for a class or function node.
fn extract_identifier(node: Node, code: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            return Some(child.utf8_text(code.as_bytes()).unwrap().to_string());
        }
    }
    None
}

/// Extracts parameters for a function node.
fn extract_parameters(node: Node, code: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "parameters" {
            return Some(child.utf8_text(code.as_bytes()).unwrap().to_string());
        }
    }
    None
}

/// Extracts methods and attributes for a class node.
fn extract_methods_and_attributes(node: Node, code: &str) -> (Option<String>, Option<String>) {
    let mut methods = Vec::new();
    let mut attributes = Vec::new();

    // Recursive function to traverse the class node
    fn traverse(node: Node, code: &str, methods: &mut Vec<String>, attributes: &mut Vec<String>) {
        if node.kind() == "function_definition" {
            if let Some(function_name) = extract_identifier(node, code) {
                if function_name == "__init__" {
                    // If `__init__` is found, extract its parameters as attributes
                    if let Some(params) = extract_parameters(node, code) {
                        attributes.extend(
                            params
                                .split(',')
                                .skip(1) // Skip "self"
                                .map(|param| {
                                    param
                                        .trim() // Remove leading/trailing whitespace
                                        .split(':') // Split by type annotation
                                        .next() // Take only the parameter name
                                        .unwrap_or("") // Default to empty if nothing found
                                        .trim_end_matches(')') // Remove trailing ')'
                                        .to_string()
                                }),
                        );
                        println!("Attributes from __init__: {:?}", attributes); // Debug output
                    }
                } else {
                    // Otherwise, add the function name to methods
                    methods.push(function_name);
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            traverse(child, code, methods, attributes); // Recursive traversal
        }
    }

    // Start traversal from the root node of the class
    traverse(node, code, &mut methods, &mut attributes);

    let methods_str = if !methods.is_empty() {
        Some(methods.join(", "))
    } else {
        None
    };

    let attributes_str = if !attributes.is_empty() {
        Some(attributes.join(", "))
    } else {
        None
    };

    (methods_str, attributes_str)
}
