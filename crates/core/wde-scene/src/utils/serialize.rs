use serde_json::Value;

const PARSER_VERSION: f32 = 1.0;

pub enum FileType {
    Config
}

pub struct FileData {
    pub file_type: FileType,
    pub path: String,
    pub label: String,
    pub description: String,
    pub data: Value
}

pub fn parse_file(path: &str) -> Result<FileData, Box<dyn std::error::Error>> {
    // Try to read the file content
    let file_content = std::fs::read_to_string(format!("./res/{}", path))?;
    let parsed: Value = serde_json::from_str(&file_content)?;

    // Check if the file is valid and has the correct version
    if parsed["version"].as_f64().unwrap_or(0.0) != PARSER_VERSION as f64 {
        return Err(format!("Unsupported file version: {}. Expected version: {}", parsed["version"], PARSER_VERSION).into());
    }

    // Parse the file type
    let file_type = match parsed["type"].as_str() {
        Some("res/config") => FileType::Config,
        _ => return Err(format!("Unsupported file type: {}", parsed["type"]).into())
    };

    // Parse the file data
    let data = FileData {
        file_type,
        path: path.to_string(),
        label: parsed["label"].as_str().unwrap_or("").to_string(),
        description: parsed["description"].as_str().unwrap_or("").to_string(),
        data: parsed["data"].clone()
    };
    Ok(data)
}

