//! Finder core library
//!
//! All business logic lives here. No platform-specific code.
//! Camera + AI vision object detection with natural language queries.

pub mod error;
pub mod models;
pub mod store;
pub mod vision;

pub use error::Error;
pub use models::*;
pub use store::Store;
pub use vision::VisionClient;
