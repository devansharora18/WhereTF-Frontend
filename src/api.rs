use serde::{Deserialize, Serialize};

const BASE_URL: &str = "http://localhost:8000";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    pub file_path: String,
    #[serde(alias = "content_text")]
    pub content: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub status: String,
    pub query: String,
    pub mode: String,
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexedFile {
    pub id: String,
    pub file_path: String,
    pub mime_type: String,
    pub tags: Vec<String>,
    pub context: Option<String>,
    pub last_modified: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub status: String,
    pub message: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataUpdate {
    pub status: String,
    pub message: String,
    pub file_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteResponse {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub service: String,
    pub database_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

pub async fn upload_file(file_path: &str) -> Result<UploadResponse, String> {
    let url = format!("{}/upload/", BASE_URL);
    let file_data = std::fs::read(file_path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    let filename = std::path::Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let part = reqwest::multipart::Part::bytes(file_data)
        .file_name(filename)
        .mime_str("application/octet-stream")
        .map_err(|e| format!("Failed to create multipart part: {}", e))?;

    let form = reqwest::multipart::Form::new().part("file", part);

    let client = reqwest::Client::new();
    client
        .post(&url)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Upload request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse upload response: {}", e))
}

pub async fn health_check() -> Result<HealthResponse, String> {
    let url = format!("{}/health", BASE_URL);
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Health check failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse health response: {}", e))
}

pub async fn search(query: &str, mode: &str, top_k: u32) -> Result<SearchResponse, String> {
    let url = format!(
        "{}/search/?query={}&mode={}&top_k={}",
        BASE_URL,
        urlencoding(query),
        urlencoding(mode),
        top_k
    );
    let client = reqwest::Client::new();
    client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Search request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse search response: {}", e))
}

pub async fn get_all_files() -> Result<Vec<IndexedFile>, String> {
    let url = format!("{}/files/", BASE_URL);
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to fetch files: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse files response: {}", e))
}

pub async fn delete_file(file_id: &str) -> Result<DeleteResponse, String> {
    let url = format!("{}/files/{}", BASE_URL, file_id);
    let client = reqwest::Client::new();
    client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("Delete request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse delete response: {}", e))
}

pub async fn update_metadata(
    file_id: &str,
    payload: &MetadataPayload,
) -> Result<MetadataUpdate, String> {
    let url = format!("{}/files/{}", BASE_URL, file_id);
    let client = reqwest::Client::new();
    client
        .patch(&url)
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("Update metadata failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse metadata response: {}", e))
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('#', "%23")
        .replace('&', "%26")
        .replace('?', "%3F")
        .replace('=', "%3D")
        .replace('+', "%2B")
}
