use system::{Document, DocumentReadable, DocumentSnapshot, FileId};
use tokio::fs;

pub async fn write_document_file(file_id: &FileId, document: &Document) {
    let file_name = create_file_name(file_id);
    let snapshot = document.snapshot();
    fs::write(file_name, snapshot.content())
        .await
        .expect("must succeed");
}

pub async fn list_document_files() -> Vec<FileId> {
    let mut result = Vec::new();

    let dir = std::env::current_dir().expect("must succeed");
    let mut entries = fs::read_dir(dir).await.expect("must succeed");
    while let Some(entry) = entries.next_entry().await.expect("must succeed") {
        let file_name = entry.file_name().into_string().expect("must succeed");
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

pub async fn get_document_file_meta(file_id: &FileId) -> Result<(), ()> {
    let file_name = create_file_name(file_id);
    if let Ok(_) = fs::metadata(file_name).await {
        Ok(())
    } else {
        Err(())
    }
}

pub async fn read_document_file(file_id: &FileId) -> Result<Document, ()> {
    let file_name = create_file_name(file_id);
    if let Ok(v) = fs::read(file_name).await {
        // TODO: DocumentSnapshot -> Document 변환시 에러 처리
        Ok(Document::from(&DocumentSnapshot::from_vec(v)))
    } else {
        Err(())
    }
}

fn create_file_name(file_id: &FileId) -> String {
    format!("{}.rcs", file_id.to_string())
}
