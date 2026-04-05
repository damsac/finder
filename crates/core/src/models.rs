use serde::{Deserialize, Serialize};

/// Normalized bounding box (0.0 to 1.0 for each coordinate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BBox {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

/// Result of a single frame detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub found: bool,
    pub confidence: f64,
    pub bbox: Option<BBox>,
}

/// A saved search result for history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub query: String,
    pub confidence: f64,
    pub bbox_x_min: f64,
    pub bbox_y_min: f64,
    pub bbox_x_max: f64,
    pub bbox_y_max: f64,
    pub created_at: String,
}
