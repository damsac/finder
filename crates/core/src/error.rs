#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("API error: {0}")]
    Api(String),

    #[error("No object found")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] rusqlite_migration::Error),
}
