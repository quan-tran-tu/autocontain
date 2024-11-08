#[derive(Debug)]
pub struct Repository {
    pub id: Option<i32>, 
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug)]
pub struct Function {
    pub id: Option<i32>,
    pub repo_id: i32,
    pub name: String,
    pub parameters: Option<String>,
    pub return_type: Option<String>,
    pub file_location: String,
    pub start_line: i32,
    pub end_line: i32,
}

#[derive(Debug)]
pub struct Class {
    pub id: Option<i32>,
    pub repo_id: i32,
    pub name: String,
    pub attributes: Option<String>,
    pub methods: Option<String>,
    pub file_location: String,
    pub start_line: i32,
    pub end_line: i32,
}
