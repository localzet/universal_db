/// Модуль Phase 6: Управление конкурентностью и параллелизмом
///
/// Включает реализацию:
/// - Асинхронные операции (Tokio)
/// - Многопоточный доступ (Arc, Mutex, RwLock)
/// - Параллельная обработка графов (Rayon)
/// - Пулы потоков для обработки событий

use crate::models::*;
use crate::store::{EventStore, GraphStore, StateStore};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use tokio::task;

/// Тип для потокобезопасного хранилища событий
pub type ConcurrentEventStore = Arc<RwLock<EventStore>>;

/// Тип для потокобезопасного хранилища состояния
pub type ConcurrentStateStore = Arc<Mutex<StateStore>>;

/// Тип для потокобезопасного графового хранилища
pub type ConcurrentGraphStore = Arc<RwLock<GraphStore>>;

/// Менеджер конкурентности для универсальной БД
pub struct ConcurrencyManager {
    pub graph: ConcurrentGraphStore,
    pub events: ConcurrentEventStore,
    pub state: ConcurrentStateStore,
    pub max_workers: usize,
}

impl ConcurrencyManager {
    /// Создать новый менеджер конкурентности
    pub fn new(max_workers: usize) -> Self {
        ConcurrencyManager {
            graph: Arc::new(RwLock::new(GraphStore::new())),
            events: Arc::new(RwLock::new(EventStore::new())),
            state: Arc::new(Mutex::new(StateStore::new())),
            max_workers,
        }
    }

    /// Добавить узел (потокобезопасно)
    pub fn add_node(&self, node: Node) {
        let mut graph = self.graph.write();
        graph.add_node(node);
    }

    /// Добавить ребро (потокобезопасно)
    pub fn add_edge(&self, edge: Edge) {
        let mut graph = self.graph.write();
        graph.add_edge(edge);
    }

    /// Обработать событие (потокобезопасно)
    pub fn process_event(&self, event: Event) {
        {
            let mut events = self.events.write();
            events.append_event(event.clone());
        }
        {
            let mut state = self.state.lock();
            state.apply_event(&event);
        }
    }

    /// Выполнить операцию чтения на графе
    pub fn read_graph<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&GraphStore) -> T,
    {
        let graph = self.graph.read();
        f(&graph)
    }

    /// Выполнить операцию письма на графе
    pub fn write_graph<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut GraphStore) -> T,
    {
        let mut graph = self.graph.write();
        f(&mut graph)
    }

    /// Получить количество ожидающих задач
    pub fn pending_tasks(&self) -> usize {
        self.events.read().len()
    }
}

/// Параллельный обработчик событий
pub struct ParallelEventProcessor {
    batch_size: usize,
    num_workers: usize,
}

impl ParallelEventProcessor {
    pub fn new(batch_size: usize, num_workers: usize) -> Self {
        ParallelEventProcessor { batch_size, num_workers }
    }

    /// Обработать события в параллельных батчах
    pub fn process_parallel(
        &self,
        events: Vec<Event>,
        state: Arc<Mutex<StateStore>>,
    ) -> Result<(), String> {
        let batches: Vec<Vec<Event>> = events
            .chunks(self.batch_size)
            .map(|chunk| chunk.to_vec())
            .collect();

        for batch in batches {
            let state = Arc::clone(&state);
            let batch = batch.clone();

            rayon::scope(|s| {
                for event in batch {
                    let state = Arc::clone(&state);
                    s.spawn(move |_| {
                        let mut st = state.lock();
                        st.apply_event(&event);
                    });
                }
            });
        }

        Ok(())
    }
}

/// Асинхронный обработчик событий с очередью
pub struct AsyncEventQueue {
    queue: Arc<tokio::sync::Mutex<Vec<Event>>>,
    state: Arc<Mutex<StateStore>>,
}

impl AsyncEventQueue {
    pub fn new(state: Arc<Mutex<StateStore>>) -> Self {
        AsyncEventQueue {
            queue: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            state,
        }
    }

    /// Добавить событие в асинхронную очередь
    pub async fn enqueue(&self, event: Event) {
        let mut queue = self.queue.lock().await;
        queue.push(event);
    }

    /// Обработать очередь событий асинхронно
    pub async fn process_queue(&self) -> Result<usize, String> {
        let mut queue = self.queue.lock().await;
        let count = queue.len();

        while let Some(event) = queue.pop() {
            let state = Arc::clone(&self.state);
            task::spawn(async move {
                let mut st = state.lock();
                st.apply_event(&event);
            });
        }

        Ok(count)
    }

    /// Получить размер очереди
    pub async fn queue_size(&self) -> usize {
        let queue = self.queue.lock().await;
        queue.len()
    }
}

/// Блокировка для чтения/записи с таймаутом
pub struct TimeoutLock<T> {
    inner: Arc<RwLock<T>>,
    timeout_ms: u64,
}

impl<T> TimeoutLock<T> {
    pub fn new(value: T, timeout_ms: u64) -> Self {
        TimeoutLock {
            inner: Arc::new(RwLock::new(value)),
            timeout_ms,
        }
    }

    /// Попытаться получить читающую блокировку
    pub fn try_read(&self) -> Option<parking_lot::RwLockReadGuard<T>> {
        self.inner.try_read()
    }

    /// Попытаться получить пишущую блокировку
    pub fn try_write(&self) -> Option<parking_lot::RwLockWriteGuard<T>> {
        self.inner.try_write()
    }
}

/// Счётчик активных операций
pub struct OperationCounter {
    counter: Arc<Mutex<usize>>,
}

impl OperationCounter {
    pub fn new() -> Self {
        OperationCounter {
            counter: Arc::new(Mutex::new(0)),
        }
    }

    pub fn increment(&self) {
        let mut c = self.counter.lock();
        *c += 1;
    }

    pub fn decrement(&self) {
        let mut c = self.counter.lock();
        if *c > 0 {
            *c -= 1;
        }
    }

    pub fn get(&self) -> usize {
        *self.counter.lock()
    }

    pub fn reset(&self) {
        let mut c = self.counter.lock();
        *c = 0;
    }
}

/// Guard для автоматического управления счётчиком операций
pub struct OperationGuard {
    counter: Arc<Mutex<usize>>,
}

impl OperationGuard {
    pub fn new(counter: Arc<Mutex<usize>>) -> Self {
        let mut c = counter.lock();
        *c += 1;
        OperationGuard { counter }
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        let mut c = self.counter.lock();
        if *c > 0 {
            *c -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrency_manager_basic() {
        let manager = ConcurrencyManager::new(4);

        let node = Node::new("n1".to_string(), vec!["Test".to_string()]);
        manager.add_node(node);

        let count = manager.read_graph(|g| g.nodes.len());
        assert_eq!(count, 1);
    }

    #[test]
    fn test_operation_counter() {
        let counter = OperationCounter::new();

        assert_eq!(counter.get(), 0);
        counter.increment();
        assert_eq!(counter.get(), 1);

        counter.decrement();
        assert_eq!(counter.get(), 0);
    }

    #[test]
    fn test_operation_guard() {
        let counter = Arc::new(Mutex::new(0usize));

        {
            let _guard = OperationGuard::new(Arc::clone(&counter));
            let c = counter.lock();
            assert_eq!(*c, 1);
        }

        let c = counter.lock();
        assert_eq!(*c, 0);
    }

    #[tokio::test]
    async fn test_async_event_queue() {
        let state = Arc::new(Mutex::new(StateStore::new()));
        let queue = AsyncEventQueue::new(state);

        let event = Event::new(
            "e1".to_string(),
            "n1".to_string(),
            EventType::CreateNode,
            serde_json::json!({"name": "Test"}),
        );

        queue.enqueue(event).await;
        assert_eq!(queue.queue_size().await, 1);
    }
}
