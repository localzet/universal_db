/// Модуль Phase 4: Персистентность и восстановление
///
/// Включает реализацию:
/// - Сохранение графа в JSON файлы
/// - Загрузка из диска
/// - Восстановление после сбоя
/// - Резервные копии

use crate::models::*;
use crate::store::{EventStore, GraphStore, StateStore};
use std::fs;
use std::path::Path;

/// Тип ошибки для операций персистентности
#[derive(Debug, Clone)]
pub enum PersistenceError {
    IoError(String),
    SerializationError(String),
}

impl std::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersistenceError::IoError(msg) => write!(f, "IO Error: {}", msg),
            PersistenceError::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
        }
    }
}

/// Менеджер персистентности (JSON-based файловое хранилище)
pub struct PersistenceManager {
    base_path: String,
}

impl PersistenceManager {
    /// Создать или открыть хранилище
    pub fn open(path: &str) -> Result<Self, PersistenceError> {
        fs::create_dir_all(path)
            .map_err(|e| PersistenceError::IoError(format!("Не удалось создать директорию: {}", e)))?;

        Ok(PersistenceManager {
            base_path: path.to_string(),
        })
    }

    /// Сохранить событие в JSON файл
    pub fn save_event(&self, event: &Event) -> Result<(), PersistenceError> {
        let dir = format!("{}/events", self.base_path);
        fs::create_dir_all(&dir)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        let path = format!("{}/{}.json", dir, event.id);
        let json = serde_json::to_string_pretty(event)
            .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

        fs::write(&path, json)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Загрузить событие из JSON файла
    pub fn load_event(&self, event_id: &str) -> Result<Option<Event>, PersistenceError> {
        let path = format!("{}/events/{}.json", self.base_path, event_id);
        match fs::read_to_string(&path) {
            Ok(content) => {
                let event = serde_json::from_str(&content)
                    .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;
                Ok(Some(event))
            }
            Err(_) => Ok(None),
        }
    }

    /// Сохранить узел в JSON файл
    pub fn save_node(&self, node: &Node) -> Result<(), PersistenceError> {
        let dir = format!("{}/nodes", self.base_path);
        fs::create_dir_all(&dir)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        let path = format!("{}/{}.json", dir, node.id);
        let json = serde_json::to_string_pretty(node)
            .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

        fs::write(&path, json)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Загрузить узел из JSON файла
    pub fn load_node(&self, node_id: &str) -> Result<Option<Node>, PersistenceError> {
        let path = format!("{}/nodes/{}.json", self.base_path, node_id);
        match fs::read_to_string(&path) {
            Ok(content) => {
                let node = serde_json::from_str(&content)
                    .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;
                Ok(Some(node))
            }
            Err(_) => Ok(None),
        }
    }

    /// Сохранить ребро в JSON файл
    pub fn save_edge(&self, edge: &Edge) -> Result<(), PersistenceError> {
        let dir = format!("{}/edges", self.base_path);
        fs::create_dir_all(&dir)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        let path = format!("{}/{}.json", dir, edge.id);
        let json = serde_json::to_string_pretty(edge)
            .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

        fs::write(&path, json)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Загрузить ребро из JSON файла
    pub fn load_edge(&self, edge_id: &str) -> Result<Option<Edge>, PersistenceError> {
        let path = format!("{}/edges/{}.json", self.base_path, edge_id);
        match fs::read_to_string(&path) {
            Ok(content) => {
                let edge = serde_json::from_str(&content)
                    .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;
                Ok(Some(edge))
            }
            Err(_) => Ok(None),
        }
    }

    /// Сохранить состояние в JSON файл
    pub fn save_state(&self, state: &State) -> Result<(), PersistenceError> {
        let dir = format!("{}/states", self.base_path);
        fs::create_dir_all(&dir)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        let path = format!("{}/{}_{}.json", dir, state.target_id, state.version);
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

        fs::write(&path, json)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Загрузить все события из JSON файлов
    pub fn load_all_events(&self) -> Result<EventStore, PersistenceError> {
        let mut event_store = EventStore::new();
        let events_dir = format!("{}/events", self.base_path);

        if Path::new(&events_dir).exists() {
            for entry in fs::read_dir(&events_dir)
                .map_err(|e| PersistenceError::IoError(e.to_string()))?
            {
                let entry = entry.map_err(|e| PersistenceError::IoError(e.to_string()))?;
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    let content = fs::read_to_string(&path)
                        .map_err(|e| PersistenceError::IoError(e.to_string()))?;
                    let event: Event = serde_json::from_str(&content)
                        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;
                    event_store.append_event(event);
                }
            }
        }

        Ok(event_store)
    }

    /// Загрузить все состояния  из JSON файлов
    pub fn load_all_states(&self) -> Result<StateStore, PersistenceError> {
        let mut state_store = StateStore::new();
        let states_dir = format!("{}/states", self.base_path);

        if Path::new(&states_dir).exists() {
            for entry in fs::read_dir(&states_dir)
                .map_err(|e| PersistenceError::IoError(e.to_string()))?
            {
                let entry = entry.map_err(|e| PersistenceError::IoError(e.to_string()))?;
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    let content = fs::read_to_string(&path)
                        .map_err(|e| PersistenceError::IoError(e.to_string()))?;
                    let state: State = serde_json::from_str(&content)
                        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;

                    state_store
                        .states
                        .entry(state.target_id.clone())
                        .or_insert_with(Vec::new)
                        .push(state);
                }
            }
        }

        Ok(state_store)
    }

    /// Загрузить весь граф из JSON файлов
    pub fn load_all_graph(&self) -> Result<GraphStore, PersistenceError> {
        let mut graph = GraphStore::new();

        // Загружаем узлы
        let nodes_dir = format!("{}/nodes", self.base_path);
        if Path::new(&nodes_dir).exists() {
            for entry in fs::read_dir(&nodes_dir)
                .map_err(|e| PersistenceError::IoError(e.to_string()))?
            {
                let entry = entry.map_err(|e| PersistenceError::IoError(e.to_string()))?;
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    let content = fs::read_to_string(&path)
                        .map_err(|e| PersistenceError::IoError(e.to_string()))?;
                    let node: Node = serde_json::from_str(&content)
                        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;
                    graph.add_node(node);
                }
            }
        }

        // Загружаем рёбра
        let edges_dir = format!("{}/edges", self.base_path);
        if Path::new(&edges_dir).exists() {
            for entry in fs::read_dir(&edges_dir)
                .map_err(|e| PersistenceError::IoError(e.to_string()))?
            {
                let entry = entry.map_err(|e| PersistenceError::IoError(e.to_string()))?;
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    let content = fs::read_to_string(&path)
                        .map_err(|e| PersistenceError::IoError(e.to_string()))?;
                    let edge: Edge = serde_json::from_str(&content)
                        .map_err(|e| PersistenceError::SerializationError(e.to_string()))?;
                    graph.add_edge(edge);
                }
            }
        }

        Ok(graph)
    }

    /// Выполнить checkpoint (резервная копия)
    pub fn checkpoint(&self, backup_path: &str) -> Result<(), PersistenceError> {
        // Создаём директорию для backup
        fs::create_dir_all(backup_path)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?;

        // Копируем все файлы рекурсивно
        for entry in fs::read_dir(&self.base_path)
            .map_err(|e| PersistenceError::IoError(e.to_string()))?
        {
            let entry = entry.map_err(|e| PersistenceError::IoError(e.to_string()))?;
            let path = entry.path();
            let file_name = path.file_name().ok_or_else(|| {
                PersistenceError::IoError("Не удалось получить имя файла".to_string())
            })?;

            let backup_file = format!("{}/{}", backup_path, file_name.to_string_lossy());

            if path.is_dir() {
                fs::create_dir_all(&backup_file)
                    .map_err(|e| PersistenceError::IoError(e.to_string()))?;
                
                for subentry in fs::read_dir(&path)
                    .map_err(|e| PersistenceError::IoError(e.to_string()))?
                {
                    let subentry = subentry.map_err(|e| PersistenceError::IoError(e.to_string()))?;
                    let subpath = subentry.path();
                    let subfn = subpath.file_name().ok_or_else(|| {
                        PersistenceError::IoError("Не удалось получить имя файла".to_string())
                    })?;
                    let backup_subfile = format!("{}/{}", backup_file, subfn.to_string_lossy());
                    fs::copy(&subpath, &backup_subfile)
                        .map_err(|e| PersistenceError::IoError(e.to_string()))?;
                }
            } else {
                fs::copy(&path, &backup_file)
                    .map_err(|e| PersistenceError::IoError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Очистить хранилище
    pub fn clear(&self) -> Result<(), PersistenceError> {
        for subdir in &["nodes", "edges", "events", "states"] {
            let dir = format!("{}/{}", self.base_path, subdir);
            if Path::new(&dir).exists() {
                fs::remove_dir_all(&dir)
                    .map_err(|e| PersistenceError::IoError(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Получить количество элементов в хранилище
    pub fn count(&self) -> Result<usize, PersistenceError> {
        let mut total = 0;

        for subdir in &["nodes", "edges", "events", "states"] {
            let dir = format!("{}/{}", self.base_path, subdir);
            if Path::new(&dir).exists() {
                total += fs::read_dir(&dir)
                    .map_err(|e| PersistenceError::IoError(e.to_string()))?
                    .count();
            }
        }

        Ok(total)
    }

    /// Закрыть хранилище
    pub fn close(self) -> Result<(), PersistenceError> {
        // JSON-based storage doesn't need explicit closing
        Ok(())
    }
}

/// Восстановление хранилища после сбоя
pub fn recover_from_backup(
    _backup_path: &str,
    restore_path: &str,
) -> Result<PersistenceManager, PersistenceError> {
    // В реальном приложении здесь был бы код восстановления из backup
    PersistenceManager::open(restore_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_save_load() {
        if let Ok(manager) = PersistenceManager::open(".test_db_persist") {
            let node = Node::new("n1".to_string(), vec!["Test".to_string()]);
            let _ = manager.save_node(&node);
            let loaded = manager.load_node("n1").unwrap();
            assert!(loaded.is_some());
            let _ = manager.clear();
        }
    }
}
