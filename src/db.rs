use rusqlite::{params, Connection, Result};
use crate::models::{Repository, Function, Class};

// Initialize the database to store information about classes, functions and their dependencies
pub fn initialize_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS repositories (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS classes (
            id INTEGER PRIMARY KEY,
            repo_id INTEGER,
            name TEXT NOT NULL,
            attributes TEXT,
            file_location TEXT,
            start_line INTEGER,
            end_line INTEGER,
            docstring TEXT,     
            FOREIGN KEY(repo_id) REFERENCES repositories(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS functions (
            id INTEGER PRIMARY KEY,
            repo_id INTEGER,
            class_id INTEGER,      
            name TEXT NOT NULL,
            parameters TEXT,            
            return_type TEXT,           
            file_location TEXT,
            start_line INTEGER,
            end_line INTEGER,
            docstring TEXT,    
            FOREIGN KEY(repo_id) REFERENCES repositories(id),
            FOREIGN KEY(class_id) REFERENCES classes(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS function_dependencies (
            function_name TEXT NOT NULL,
            dependency TEXT NOT NULL,
            class_id INTEGER,
            FOREIGN KEY(class_id) REFERENCES classes(id)
        )",
        [],
    )?;

    Ok(())
}


//---------------- List of functions to interact with the sqlite database -----------------

// Add repository to database
pub fn insert_repository(conn: &Connection, repo: &Repository) -> Result<i32> {
    conn.execute(
        "INSERT INTO repositories (name, description) VALUES (?1, ?2)",
        &[&repo.name, &repo.description.as_deref().unwrap_or("").to_string()],
    )?;
    let repo_id = conn.last_insert_rowid() as i32;
    Ok(repo_id)
}

// Add function to database
pub fn insert_function(conn: &Connection, func: &Function) -> Result<()> {
    conn.execute(
        "INSERT INTO functions (repo_id, class_id, name, parameters, return_type, file_location, start_line, end_line, docstring)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            func.repo_id,
            func.class_id,
            func.name,
            func.parameters,
            func.return_type,
            func.file_location,
            func.start_line,
            func.end_line,
            func.docstring
        ],
    )?;
    Ok(())
}

// Add class to database
pub fn insert_class(conn: &Connection, class: &Class) -> Result<()> {
    conn.execute(
        "INSERT INTO classes (repo_id, name, attributes, file_location, start_line, end_line, docstring)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            class.repo_id,
            class.name,
            class.attributes,
            class.file_location,
            class.start_line,
            class.end_line,
            class.docstring
        ],
    )?;
    Ok(())
}

// Add function dependencies to database
pub fn insert_dependencies(conn: &Connection, function_name: &str, class_id: Option<i32>, dependencies: &[String]) -> Result<()> {
    let mut stmt = conn.prepare("INSERT INTO function_dependencies (function_name, dependency, class_id) VALUES (?1, ?2, ?3)")?;

    for dependency in dependencies {
        stmt.execute(params![function_name, dependency, class_id])?;
    }

    Ok(())
}

// Fetch dependencies for a specific function and class ID, if applicable
pub fn get_dependencies(conn: &Connection, function_name: &str, class_id: Option<i32>) -> Result<Vec<(String, Option<i32>)>> {
    let mut stmt = if class_id.is_some() {
        conn.prepare("SELECT dependency, class_id FROM function_dependencies WHERE function_name = ?1 AND class_id = ?2")?
    } else {
        conn.prepare("SELECT dependency, class_id FROM function_dependencies WHERE function_name = ?1 AND class_id IS NULL")?
    };

    let dependencies = if let Some(class_id) = class_id {
        stmt.query_map(params![function_name, class_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect::<Result<Vec<_>, _>>()? // Collect results in this branch
    } else {
        stmt.query_map(params![function_name], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })?.collect::<Result<Vec<_>, _>>()? // Collect results in this branch
    };

    Ok(dependencies)
}

// Fetch description (docstring) for a specific function, with an optional class ID
pub fn get_function_description(conn: &Connection, function_name: &str, class_id: Option<i32>) -> Result<String> {
    let mut stmt = if class_id.is_some() {
        conn.prepare("SELECT docstring FROM functions WHERE name = ?1 AND class_id = ?2")?
    } else {
        conn.prepare("SELECT docstring FROM functions WHERE name = ?1 AND class_id IS NULL")?
    };

    if let Some(class_id) = class_id {
        stmt.query_row(params![function_name, class_id], |row| row.get(0))
    } else {
        stmt.query_row(params![function_name], |row| row.get(0))
    }.map_err(|e| e.into())
}
