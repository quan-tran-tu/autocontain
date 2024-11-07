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
    Ok(())
}

pub fn insert_repository(conn: &Connection, repo: &Repository) -> Result<i32> {
    conn.execute(
        "INSERT INTO repositories (name, description) VALUES (?1, ?2)",
        &[&repo.name, &repo.description.as_deref().unwrap_or("").to_string()],
    )?;
    // Retrieve the last inserted row ID (which will be the repo_id)
    let repo_id = conn.last_insert_rowid() as i32;
    Ok(repo_id)
}

pub fn insert_function(conn: &Connection, func: &Function) -> Result<()> {
    conn.execute(
        "INSERT INTO functions (repo_id, name, parameters, file_location, start_line, end_line) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![func.repo_id, func.name, func.parameters, func.file_location, func.start_line, func.end_line],
    )?;
    Ok(())
}

pub fn insert_class(conn: &Connection, class: &Class) -> Result<()> {
    conn.execute(
        "INSERT INTO classes (repo_id, name, attributes, methods, file_location, start_line, end_line) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![class.repo_id, class.name, class.attributes, class.methods, class.file_location, class.start_line, class.end_line],
    )?;
    Ok(())
}
