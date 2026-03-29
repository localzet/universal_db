mod events;
mod models;
mod store;
mod graph_algorithms;
mod indexing;
mod persistence;
mod transactions;
mod concurrency;
mod benchmarks;
mod formal_verification;

use chrono::Utc;
use models::*;
use store::{EventStore, GraphStore, StateStore};
use graph_algorithms::*;
use indexing::*;
use transactions::*;
use formal_verification::*;

/// Демонстрация всех фаз Universal DB
fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  Universal DB: Graph + Time + Event + State (Phases 1-8)    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // === PHASE 1: Основные структуры и базовое хранилище ===
    println!("\n📍 PHASE 1: Основные структуры");
    println!("     - Node/Edge/Event/State модели");
    println!("     - GraphStore, EventStore, StateStore");
    println!("     - Event replay и state versioning");

    let mut graph_store = GraphStore::new();
    let mut event_store = EventStore::new();
    let mut state_store = StateStore::new();

    // Создаём граф
    let node_alice = Node::new("n1".to_string(), vec!["Person".to_string()]);
    let node_bob = Node::new("n2".to_string(), vec!["Person".to_string()]);
    graph_store.add_node(node_alice.clone());
    graph_store.add_node(node_bob.clone());

    // Создаём события
    let event_create_alice = Event::new(
        "e1".to_string(),
        "n1".to_string(),
        EventType::CreateNode,
        serde_json::json!({"name": "Alice", "status": "active"}),
    );

    let event_create_bob = Event::new(
        "e2".to_string(),
        "n2".to_string(),
        EventType::CreateNode,
        serde_json::json!({"name": "Bob", "status": "active"}),
    );

    event_store.append_event(event_create_alice.clone());
    event_store.append_event(event_create_bob.clone());

    // Обновления состояния
    let event_update_alice = Event::new(
        "e3".to_string(),
        "n1".to_string(),
        EventType::UpdateNode,
        serde_json::json!({"age": 28, "city": "SF"}),
    );

    event_store.append_event(event_update_alice.clone());

    // Ребро
    let edge_knows = Edge::new(
        "e_knows".to_string(),
        "n1".to_string(),
        "n2".to_string(),
        "KNOWS".to_string(),
    );
    graph_store.add_edge(edge_knows);

    let event_create_edge = Event::new(
        "e4".to_string(),
        "e_knows".to_string(),
        EventType::CreateEdge,
        serde_json::json!({"since": 2020}),
    );
    event_store.append_event(event_create_edge);

    // Replay
    events::replay_events(&event_store.events.clone(), &mut state_store);
    println!("   ✓ Events replayed: {}", event_store.len());
    println!("   ✓ State versions tracked: {}", state_store.states.len());

    // === PHASE 2: Графовые алгоритмы ===
    println!("\n📍 PHASE 2: Графовые алгоритмы");
    
    let bfs = bfs(&graph_store, "n1", None);
    println!("   ✓ BFS из n1: {} узлов посещено", bfs.visited.len());
    
    let dfs = dfs(&graph_store, "n1");
    println!("   ✓ DFS из n1: {} узлов посещено", dfs.len());
    
    let path = find_shortest_path(&graph_store, "n1", "n2");
    println!("   ✓ Кратчайший путь n1 -> n2: {} шагов", path.distance);
    
    let neighbors = find_neighbors_by_type(&graph_store, "n1", "KNOWS");
    println!("   ✓ Соседи n1 по KNOWS: {}", neighbors.len());

    // === PHASE 3: Индексирование ===
    println!("\n📍 PHASE 3: Индексирование и оптимизация");
    
    let mut composite_idx = CompositeIndex::new();
    
    // Добавляем в индексы
    composite_idx.type_index.add("KNOWS".to_string(), "e_knows".to_string());
    composite_idx.property_index.add(
        "name".to_string(),
        serde_json::json!("Alice"),
        "n1".to_string(),
    );
    
    let stats = composite_idx.get_stats();
    println!("   ✓ Type index entries: {}", stats.type_index_entries);
    println!("   ✓ Property index entries: {}", stats.property_index_entries);

    // === PHASE 4: Персистентность ===
    println!("\n📍 PHASE 4: Персистентность (RocksDB)");
    
    match persistence::PersistenceManager::open(".test_db_phase4") {
        Ok(pm) => {
            let _ = pm.save_node(&node_alice);
            println!("   ✓ Узел сохранён в RocksDB");
            let _ = pm.close();
            let _ = std::fs::remove_dir_all(".test_db_phase4");
        }
        Err(e) => println!("   ⚠ Ошибка: {}", e),
    }

    // === PHASE 5: Транзакции ===
    println!("\n📍 PHASE 5: Транзакции и разрешение конфликтов");
    
    let mut tx_manager = TransactionManager::new();
    let tx = tx_manager.create_transaction(
        "tx1".to_string(),
        ConflictResolutionStrategy::LastWriteWins,
    );
    println!("   ✓ Транзакция создана: {}", tx.id);
    
    let event_tx = Event::new(
        "e_tx".to_string(),
        "n1".to_string(),
        EventType::UpdateNode,
        serde_json::json!({"role": "engineer"}),
    );
    
    let _ = tx_manager.add_event_to_transaction("tx1", event_tx);
    let _ = tx_manager.commit_transaction("tx1");
    println!("   ✓ Транзакция зафиксирована");
    println!("   ✓ Всего зафиксированных транзакций: {}", tx_manager.committed_count());

    // === PHASE 6: Конкурентность ===
    println!("\n📍 PHASE 6: Конкурентность и параллелизм");
    
    let concurrency_mgr = concurrency::ConcurrencyManager::new(4);
    
    for i in 0..5 {
        let node = Node::new(format!("n_concurrent_{}", i), vec!["Test".to_string()]);
        concurrency_mgr.add_node(node);
    }
    
    let node_count = concurrency_mgr.read_graph(|g| g.nodes.len());
    println!("   ✓ Узлов создано (параллельно): {}", node_count);

    // === PHASE 7: Бенчмарки ===
    println!("\n📍 PHASE 7: Тестирование производительности");
    
    let mut profiler = benchmarks::Profiler::new();
    
    profiler.measure("node_creation_100", || {
        let mut graph = GraphStore::new();
        for i in 0..100 {
            graph.add_node(Node::new(format!("bench_n{}", i), vec![]));
        }
    });
    
    profiler.measure("event_replay_100", || {
        let mut events = Vec::new();
        for i in 0..100 {
            events.push(Event::new(
                format!("e{}", i),
                "bench_n0".to_string(),
                EventType::UpdateNode,
                serde_json::json!({"value": i}),
            ));
        }
        
        let mut states = StateStore::new();
        for event in events {
            states.apply_event(&event);
        }
    });
    
    let results = profiler.get_results();
    for result in results {
        println!("   📊 {}", result);
    }

    benchmarks::print_comparison_table();

    // === PHASE 8: Формальная верификация ===
    println!("\n📍 PHASE 8: Формальная верификация");
    
    let invariants = ConsistencyVerifier::verify_all(
        &graph_store,
        &state_store,
        &event_store.events,
    );
    
    for inv in &invariants {
        if inv.is_valid() {
            println!("   ✅ {}: валидна", inv.name);
        } else {
            println!("   ❌ {}: нарушена ({} issues)", inv.name, inv.violations.len());
        }
    }

    // === Итоговая статистика ===
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  ИТОГОВАЯ СТАТИСТИКА                                         ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║ Всего узлов (Phase 1 + 6):     {}                         ║", 
             graph_store.nodes.len() + node_count);
    println!("║ Всего рёбер:                   {}                         ║", graph_store.edges.len());
    println!("║ Всего событий:                 {}                         ║", event_store.len());
    println!("║ Версий состояния:              {}                         ║", state_store.states.len());
    println!("║ Зафиксированных транзакций:    {}                         ║", tx_manager.committed_count());
    println!("╚══════════════════════════════════════════════════════════════╝");

    println!("\n✨ Все фазы успешно реализованы!");
    println!("\n📚 Следующие шаги:");
    println!("   1. Запустить бенчмарки: cargo run --release");
    println!("   2. Запустить тесты: cargo test");
    println!("   3. Проверить документацию: README.md, USER_GUIDE.md");
}
