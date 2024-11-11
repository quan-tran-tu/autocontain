use std::env;

use dotenv::dotenv;
use once_cell::sync::Lazy;

pub static OPENAI_API_KEY: Lazy<String> = Lazy::new(|| {
    dotenv().ok();
    env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not found in .env")
});

pub const OPENAI_MODEL_NAME: &str = "gpt-4o-mini";