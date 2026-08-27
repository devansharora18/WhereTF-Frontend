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
    #[serde(default)]
    pub expanded_query: Option<String>,
    #[serde(default)]
    pub search_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IndexedFile {
    pub id: String,
    pub file_path: String,
    pub mime_type: String,
    pub tags: Vec<String>,
    pub context: Option<String>,
    pub last_modified: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FilesWrapper {
    pub status: String,
    pub count: usize,
    pub data: Vec<IndexedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResponse {
    pub status: String,
    pub message: String,
    pub filename: String,
    #[serde(default)]
    pub task_id: Option<String>,
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
pub struct RelatedFile {
    pub file_id: String,
    pub file_path: String,
    pub similarity_score: f64,
    pub relation_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchFolderResponse {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub folder_path: Option<String>,
    #[serde(default)]
    pub id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WatchedFolder {
    pub id: i64,
    pub folder_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCheckResult {
    pub needs_indexing: bool,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeedsIndexingRequest {
    pub file_path: String,
    pub file_hash: String,
}

pub async fn search(query: &str, mode: &str, top_k: u32) -> Result<SearchResponse, String> {
    let url = format!(
        "{}/search/normal/?query={}&mode={}&top_k={}",
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

pub async fn power_search(query: &str, mode: &str, top_k: u32) -> Result<SearchResponse, String> {
    let url = format!(
        "{}/search/power/?query={}&mode={}&top_k={}",
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
        .map_err(|e| format!("Power search request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse power search response: {}", e))
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

    let file_part = reqwest::multipart::Part::bytes(file_data)
        .file_name(filename)
        .mime_str("application/octet-stream")
        .map_err(|e| format!("Failed to create multipart part: {}", e))?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("original_path", file_path.to_string());

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

pub async fn get_all_files() -> Result<Vec<IndexedFile>, String> {
    let url = format!("{}/files/", BASE_URL);
    let wrapper: FilesWrapper = reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to fetch files: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse files response: {}", e))?;
    Ok(wrapper.data)
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

pub async fn delete_file_by_path(file_path: &str) -> Result<DeleteResponse, String> {
    let url = format!("{}/files/delete-by-path", BASE_URL);
    let client = reqwest::Client::new();
    client
        .post(&url)
        .json(&serde_json::json!({ "file_path": file_path }))
        .send()
        .await
        .map_err(|e| format!("Delete by path failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn rename_file(old_path: &str, new_path: &str) -> Result<DeleteResponse, String> {
    let url = format!("{}/files/rename", BASE_URL);
    let client = reqwest::Client::new();
    client
        .post(&url)
        .json(&serde_json::json!({ "old_path": old_path, "new_path": new_path }))
        .send()
        .await
        .map_err(|e| format!("Rename failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn get_related_files(file_id: &str) -> Result<Vec<RelatedFile>, String> {
    let url = format!("{}/files/{}/related", BASE_URL, file_id);
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to fetch related files: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn add_watch_folder(folder_path: &str) -> Result<WatchFolderResponse, String> {
    let url = format!("{}/watch/folder", BASE_URL);
    let client = reqwest::Client::new();
    client
        .post(&url)
        .json(&serde_json::json!({ "folder_path": folder_path }))
        .send()
        .await
        .map_err(|e| format!("Watch folder failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn remove_watch_folder(folder_path: &str) -> Result<WatchFolderResponse, String> {
    let url = format!("{}/watch/folder", BASE_URL);
    let client = reqwest::Client::new();
    client
        .delete(&url)
        .json(&serde_json::json!({ "folder_path": folder_path }))
        .send()
        .await
        .map_err(|e| format!("Remove watch folder failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn get_watched_folders() -> Result<Vec<WatchedFolder>, String> {
    let url = format!("{}/watch/folders", BASE_URL);
    reqwest::get(&url)
        .await
        .map_err(|e| format!("Failed to fetch watched folders: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))
}

pub async fn needs_indexing(file_path: &str, file_hash: &str) -> Result<bool, String> {
    let url = format!("{}/files/needs-indexing", BASE_URL);
    let client = reqwest::Client::new();
    let resp: FileCheckResult = client
        .post(&url)
        .json(&NeedsIndexingRequest {
            file_path: file_path.to_string(),
            file_hash: file_hash.to_string(),
        })
        .send()
        .await
        .map_err(|e| format!("needs_indexing request failed: {}", e))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse needs_indexing response: {}", e))?;
    Ok(resp.needs_indexing)
}

fn urlencoding(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('#', "%23")
        .replace('&', "%26")
        .replace('?', "%3F")
        .replace('=', "%3D")
        .replace('+', "%2B")
}
