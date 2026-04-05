use crate::error::Error;
use crate::models::DetectionResult;
use base64::Engine;
use serde_json::json;

pub struct VisionClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl VisionClient {
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: "anthropic/claude-sonnet-4-20250514".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Analyze a JPEG frame looking for the queried object.
    /// Returns a DetectionResult with normalized bounding box coordinates.
    pub async fn detect(&self, query: &str, jpeg_bytes: &[u8]) -> Result<DetectionResult, Error> {
        let b64 = base64::engine::general_purpose::STANDARD.encode(jpeg_bytes);
        let image_url = format!("data:image/jpeg;base64,{}", b64);

        let prompt = format!(
            r#"You are an object detection system. The user is looking for: "{}"

Analyze this image carefully. If you can see the described object, estimate its bounding box location.

Respond with ONLY this JSON (no markdown, no explanation):
{{"found": true, "confidence": 0.85, "bbox": {{"x_min": 0.1, "y_min": 0.2, "x_max": 0.4, "y_max": 0.5}}}}

Rules:
- Coordinates are normalized 0.0 to 1.0 (0,0 = top-left, 1,1 = bottom-right)
- confidence is 0.0 to 1.0
- If the object is NOT visible, respond: {{"found": false, "confidence": 0.0, "bbox": null}}
- Be conservative — only report found if you're reasonably confident"#,
            query
        );

        let body = json!({
            "model": self.model,
            "messages": [{
                "role": "user",
                "content": [
                    {
                        "type": "image_url",
                        "image_url": { "url": image_url }
                    },
                    {
                        "type": "text",
                        "text": prompt
                    }
                ]
            }],
            "max_tokens": 200
        });

        let resp = self
            .client
            .post("https://api.ppq.ai/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(Error::Api(format!("HTTP {}: {}", status, text)));
        }

        let resp_json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| Error::Network(e.to_string()))?;

        // Extract the content from OpenAI-compatible response
        let content = resp_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| Error::Api("No content in response".to_string()))?;

        // Parse the JSON response — strip any markdown fences if present
        let clean = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let result: DetectionResult = serde_json::from_str(clean)?;
        Ok(result)
    }
}
