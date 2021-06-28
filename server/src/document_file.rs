use system::FileId;
use tokio::fs;

pub async fn write_document_file(file_id: &FileId) {
    let file_name = create_file_name(file_id);
    fs::write(file_name, b"hello world").await.unwrap();
}

pub async fn list_document_files() -> Vec<FileId> {
    let mut result = Vec::new();

    let dir = std::env::current_dir().unwrap();
    let mut entries = fs::read_dir(dir).await.unwrap();
    while let Some(entry) = entries.next_entry().await.unwrap() {
        let file_name = entry.file_name().into_string().unwrap();
        if file_name.ends_with(".rcs") {
            if let Some(file_id) = file_name
                .split(".")
                .take(1)
                .next()
                .and_then(|s| s.parse::<FileId>().ok())
            {
                result.push(file_id);
            }
        }
    }

    result
}

pub async fn get_document_file(file_id: &FileId) -> Result<(), ()> {
    let file_name = create_file_name(file_id);
    if let Ok(_) = fs::metadata(file_name).await {
        Ok(())
    } else {
        Err(())
    }
}

fn create_file_name(file_id: &FileId) -> String {
    format!("{}.rcs", file_id.to_string())
}