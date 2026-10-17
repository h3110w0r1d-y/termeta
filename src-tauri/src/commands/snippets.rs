use crate::state::{AppState, owner, storage_error};
use tauri::{State, WebviewWindow};
use terminal_core::Result;
use terminal_storage::{Snippet, SnippetDraft};

#[tauri::command]
pub async fn list_snippets(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Vec<Snippet>> {
    owner(&window)?;
    state.store.list_snippets().await.map_err(storage_error)
}

#[tauri::command]
pub async fn save_snippet(
    window: WebviewWindow,
    state: State<'_, AppState>,
    draft: SnippetDraft,
) -> Result<Snippet> {
    owner(&window)?;
    state.store.save_snippet(draft).await.map_err(storage_error)
}

#[tauri::command]
pub async fn delete_snippet(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
    revision: i64,
) -> Result<()> {
    owner(&window)?;
    state
        .store
        .delete_snippet(&id, revision)
        .await
        .map_err(storage_error)
}
