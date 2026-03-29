/// Universal DB - Универсальная база данных объединяющая:
/// - Graph (граф узлов и рёбер)  
/// - Time (временные данные и версионирование)
/// - Event (события и replay)
/// - State (состояния объектов)
///
/// Phase 1-8: Основные модули
/// - Phase 1: Модели данных и базовое хранилище
/// - Phase 2: Графовые алгоритмы (BFS, DFS, path finding)
/// - Phase 3: Индексирование и оптимизация запросов
/// - Phase 4: Персистентность (RocksDB)
/// - Phase 5: Транзакции и разрешение конфликтов
/// - Phase 6: Конкурентность и параллелизм
/// - Phase 7: Бенчмарки и профилирование
/// - Phase 8: Формальная верификация

pub mod models;
pub mod store;
pub mod events;
pub mod graph_algorithms;
pub mod indexing;
pub mod persistence;
pub mod transactions;
pub mod concurrency;
pub mod benchmarks;
pub mod formal_verification;

pub use models::*;
pub use store::*;
pub use events::*;
pub use graph_algorithms::*;
pub use indexing::*;
pub use persistence::*;
pub use transactions::*;
pub use concurrency::*;
pub use benchmarks::*;
pub use formal_verification::*;
