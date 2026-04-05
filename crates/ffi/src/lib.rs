uniffi::setup_scaffolding!();

use finder_core::{Error as CoreError, SearchResult, Store, VisionClient};
use std::sync::Mutex;
use tokio::runtime::Runtime;

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum FfiError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<CoreError> for FfiError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::NotFound(msg) => FfiError::NotFound(msg),
            CoreError::Api(msg) => FfiError::Api(msg),
            CoreError::Network(msg) => FfiError::Network(msg),
            CoreError::InvalidInput(msg) => FfiError::InvalidInput(msg),
            other => FfiError::Internal(other.to_string()),
        }
    }
}

/// Bounding box result exposed to Swift
#[derive(uniffi::Record)]
pub struct FfiBBox {
    pub x_min: f64,
    pub y_min: f64,
    pub x_max: f64,
    pub y_max: f64,
}

/// Detection result exposed to Swift
#[derive(uniffi::Record)]
pub struct FfiDetectionResult {
    pub found: bool,
    pub confidence: f64,
    pub bbox_x_min: f64,
    pub bbox_y_min: f64,
    pub bbox_x_max: f64,
    pub bbox_y_max: f64,
}

/// Search history item exposed to Swift
#[derive(uniffi::Record)]
pub struct FfiSearchResult {
    pub id: String,
    pub query: String,
    pub confidence: f64,
    pub bbox_x_min: f64,
    pub bbox_y_min: f64,
    pub bbox_x_max: f64,
    pub bbox_y_max: f64,
    pub created_at: String,
}

impl From<SearchResult> for FfiSearchResult {
    fn from(r: SearchResult) -> Self {
        FfiSearchResult {
            id: r.id,
            query: r.query,
            confidence: r.confidence,
            bbox_x_min: r.bbox_x_min,
            bbox_y_min: r.bbox_y_min,
            bbox_x_max: r.bbox_x_max,
            bbox_y_max: r.bbox_y_max,
            created_at: r.created_at,
        }
    }
}

/// Main app object exposed to Swift via UniFFI
#[derive(uniffi::Object)]
pub struct AppCore {
    vision: VisionClient,
    store: Mutex<Store>,
    rt: Runtime,
}

#[uniffi::export]
impl AppCore {
    /// Create a new AppCore. Instant — no network calls.
    #[uniffi::constructor]
    pub fn new(api_key: String, data_dir: String) -> Result<Self, FfiError> {
        let rt = Runtime::new().map_err(|e| FfiError::Internal(e.to_string()))?;
        let store = Store::new(&data_dir).map_err(FfiError::from)?;
        let vision = VisionClient::new(&api_key);
        Ok(Self {
            vision,
            store: Mutex::new(store),
            rt,
        })
    }

    /// Detect an object in a JPEG frame.
    /// Returns detection result with normalized bbox coordinates.
    /// bbox values are 0.0 if not found.
    pub fn detect(
        &self,
        query: String,
        jpeg_bytes: Vec<u8>,
    ) -> Result<FfiDetectionResult, FfiError> {
        let result = self
            .rt
            .block_on(self.vision.detect(&query, &jpeg_bytes))
            .map_err(FfiError::from)?;

        let (x_min, y_min, x_max, y_max) = match &result.bbox {
            Some(b) => (b.x_min, b.y_min, b.x_max, b.y_max),
            None => (0.0, 0.0, 0.0, 0.0),
        };

        // Save successful detections to history
        if result.found {
            if let Ok(store) = self.store.lock() {
                let _ = store.save_result(&query, result.confidence, x_min, y_min, x_max, y_max);
            }
        }

        Ok(FfiDetectionResult {
            found: result.found,
            confidence: result.confidence,
            bbox_x_min: x_min,
            bbox_y_min: y_min,
            bbox_x_max: x_max,
            bbox_y_max: y_max,
        })
    }

    /// Get search history
    pub fn search_history(&self, limit: u32) -> Result<Vec<FfiSearchResult>, FfiError> {
        let store = self.store.lock().unwrap();
        let results = store.list_results(limit).map_err(FfiError::from)?;
        Ok(results.into_iter().map(FfiSearchResult::from).collect())
    }
}
