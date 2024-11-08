use rusqlite::{params, Connection, Result};
use crate::models::{Repository, Function, Class};

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
        "CREATE TABLE IF NOT EXISTS functions (
            id INTEGER PRIMARY KEY,
            repo_id INTEGER,
            name TEXT NOT NULL,
            parameters TEXT,            
            return_type TEXT,           
            file_location TEXT,
            start_line INTEGER,
            end_line INTEGER,
            FOREIGN KEY(repo_id) REFERENCES repositories(id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS classes (
            id INTEGER PRIMARY KEY,
            repo_id INTEGER,
            name TEXT NOT NULL,
            attributes TEXT,            
            methods TEXT,
            file_location TEXT,
            start_line INTEGER,
            end_line INTEGER,
            FOREIGN KEY(repo_id) REFERENCES repositories(id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS function_dependencies (
            function_name TEXT NOT NULL,
            dependency TEXT NOT NULL
        )",
        [],
    )?;

    Ok(())
}

pub fn insert_repository(conn: &Connection, repo: &Repository) -> Result<i32> {
    conn.execute(
        "INSERT INTO repositories (name, description) VALUES (?1, ?2)",
        &[&repo.name, &repo.description.as_deref().unwrap_or("").to_string()],
    )?;
    let repo_id = conn.last_insert_rowid() as i32;
    Ok(repo_id)
}

pub fn insert_function(conn: &Connection, func: &Function) -> Result<()> {
    conn.execute(
        "INSERT INTO functions (repo_id, name, parameters, return_type, file_location, start_line, end_line)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![func.repo_id, func.name, func.parameters, func.return_type, func.file_location, func.start_line, func.end_line],
    )?;
    Ok(())
}

pub fn insert_class(conn: &Connection, class: &Class) -> Result<()> {
    conn.execute(
        "INSERT INTO classes (repo_id, name, attributes, methods, file_location, start_line, end_line)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![class.repo_id, class.name, class.attributes, class.methods, class.file_location, class.start_line, class.end_line],
    )?;
    Ok(())
}

pub fn insert_dependencies(conn: &Connection, function_name: &str, dependencies: &[String]) -> Result<()> {
    let mut stmt = conn.prepare("INSERT INTO function_dependencies (function_name, dependency) VALUES (?1, ?2)")?;

    for dependency in dependencies {
        stmt.execute(params![function_name, dependency])?;
    }

    Ok(())
}
