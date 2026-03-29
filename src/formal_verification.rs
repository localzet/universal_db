/// Модуль Phase 8: Формальная верификация
///
/// Включает реализацию:
/// - Инварианты консистентности
/// - Проверки логичности
/// - Трассировка состояний
/// - Логирование для анализа в TLA+

use crate::models::*;
use crate::store::{GraphStore, StateStore};
use std::collections::HashSet;

/// Инварианты для проверки консистентности
pub struct ConsistencyInvariant {
    pub name: String,
    pub violations: Vec<String>,
}

impl ConsistencyInvariant {
    pub fn new(name: &str) -> Self {
        ConsistencyInvariant {
            name: name.to_string(),
            violations: Vec::new(),
        }
    }

    pub fn add_violation(&mut self, msg: String) {
        self.violations.push(msg);
    }

    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn print_report(&self) {
        println!("\n🔍 Проверка инварианта: {}", self.name);
        if self.is_valid() {
            println!("   ✅ Валидна");
        } else {
            println!("   ❌ Нарушена:");
            for violation in &self.violations {
                println!("   - {}", violation);
            }
        }
    }
}

/// Верификатор консистентности БД
pub struct ConsistencyVerifier;

impl ConsistencyVerifier {
    /// Проверить, что все рёбра указывают на существующие узлы
    pub fn verify_referential_integrity(graph: &GraphStore) -> ConsistencyInvariant {
        let mut invariant = ConsistencyInvariant::new("Referential Integrity");

        for edge in graph.edges.values() {
            if !graph.nodes.contains_key(&edge.from) {
                invariant.add_violation(format!(
                    "Ребро {} ссылается на несуществующий начальный узел {}",
                    edge.id, edge.from
                ));
            }
            if !graph.nodes.contains_key(&edge.to) {
                invariant.add_violation(format!(
                    "Ребро {} ссылается на несуществующий конечный узел {}",
                    edge.id, edge.to
                ));
            }
        }

        invariant
    }

    /// Проверить отсутствие циклических зависимостей в состянии
    pub fn verify_no_cycles(graph: &GraphStore) -> ConsistencyInvariant {
        let mut invariant = ConsistencyInvariant::new("No Cycles");
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_id in graph.nodes.keys() {
            if !visited.contains(node_id) {
                Self::dfs_check_cycle(
                    graph,
                    node_id,
                    &mut visited,
                    &mut rec_stack,
                    &mut invariant,
                );
            }
        }

        invariant
    }

    fn dfs_check_cycle(
        graph: &GraphStore,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        invariant: &mut ConsistencyInvariant,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        for edge in graph.get_neighbors(node) {
            if rec_stack.contains(&edge.to) {
                invariant.add_violation(format!(
                    "Обнаружен цикл: {} -> {}",
                    node, edge.to
                ));
            } else if !visited.contains(&edge.to) {
                Self::dfs_check_cycle(
                    graph,
                    &edge.to,
                    visited,
                    rec_stack,
                    invariant,
                );
            }
        }

        rec_stack.remove(node);
    }

    /// Проверить, что версии состояния монотонно возрастают
    pub fn verify_monotonic_versioning(state_store: &StateStore) -> ConsistencyInvariant {
        let mut invariant = ConsistencyInvariant::new("Monotonic Versioning");

        for (_, history) in &state_store.states {
            for i in 1..history.len() {
                if history[i].version <= history[i - 1].version {
                    invariant.add_violation(format!(
                        "Версии не возрастают монотонно: v{} -> v{}",
                        history[i - 1].version, history[i].version
                    ));
                }
            }
        }

        invariant
    }

    /// Проверить, что timestamps возрастают
    pub fn verify_temporal_ordering(state_store: &StateStore) -> ConsistencyInvariant {
        let mut invariant = ConsistencyInvariant::new("Temporal Ordering");

        for (_, history) in &state_store.states {
            for i in 1..history.len() {
                if history[i].timestamp < history[i - 1].timestamp {
                    invariant.add_violation(format!(
                        "Timestamps нарушают порядок возрастания"
                    ));
                }
            }
        }

        invariant
    }

    /// Проверить детерминированность replay
    pub fn verify_replay_determinism(
        events: &[Event],
        state_store: &StateStore,
    ) -> ConsistencyInvariant {
        let mut invariant = ConsistencyInvariant::new("Replay Determinism");

        // Создаём второе хранилище и повторно воспроизводим все события
        let mut state_store_2 = StateStore::new();
        for event in events {
            state_store_2.apply_event(event);
        }

        // Сравниваем состояния
        for key in state_store.states.keys() {
            let state_1 = state_store.current_state(key);
            let state_2 = state_store_2.current_state(key);

            match (state_1, state_2) {
                (Some(s1), Some(s2)) => {
                    if s1.properties != s2.properties {
                        invariant.add_violation(
                            format!("Состояния {} не совпадают при повторном replay", key)
                        );
                    }
                }
                (Some(_), None) | (None, Some(_)) => {
                    invariant.add_violation(
                        format!("Состояние {} присутствует только в одном хранилище", key)
                    );
                }
                _ => {}
            }
        }

        invariant
    }

    /// Выполнить полный набор проверок
    pub fn verify_all(
        graph: &GraphStore,
        state_store: &StateStore,
        events: &[Event],
    ) -> Vec<ConsistencyInvariant> {
        vec![
            Self::verify_referential_integrity(graph),
            Self::verify_no_cycles(graph),
            Self::verify_monotonic_versioning(state_store),
            Self::verify_temporal_ordering(state_store),
            Self::verify_replay_determinism(events, state_store),
        ]
    }
}

/// Трассировщик состояний для отладки
pub struct StateTracer {
    traces: Vec<TraceEntry>,
}

#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub timestamp: String,
    pub operation: String,
    pub object_id: String,
    pub state_before: String,
    pub state_after: String,
}

impl StateTracer {
    pub fn new() -> Self {
        StateTracer {
            traces: Vec::new(),
        }
    }

    pub fn trace(
        &mut self,
        operation: &str,
        object_id: &str,
        state_before: &str,
        state_after: &str,
    ) {
        self.traces.push(TraceEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            operation: operation.to_string(),
            object_id: object_id.to_string(),
            state_before: state_before.to_string(),
            state_after: state_after.to_string(),
        });
    }

    pub fn print_trace(&self) {
        println!("\n📋 Трассировка состояний:");
        println!("╔═══════════════╦══════════════╦═══════════╦═════════════════════╗");
        println!("║ Операция      ║ Объект       ║ До        ║ После               ║");
        println!("╠═══════════════╬══════════════╬═══════════╬═════════════════════╣");

        for entry in &self.traces {
            println!(
                "║ {:<13} ║ {:<12} ║ {:<9} ║ {:<19} ║",
                entry.operation, entry.object_id, entry.state_before, entry.state_after
            );
        }

        println!("╚═══════════════╩══════════════╩═══════════╩═════════════════════╝");
    }

    pub fn get_trace_log(&self) -> Vec<TraceEntry> {
        self.traces.clone()
    }

    pub fn clear(&mut self) {
        self.traces.clear();
    }
}

/// TLA+ спецификация (комментарий)
pub const TLA_SPEC: &str = r#"
--- TLA+ спецификация Universal DB ---

MODULE UniversalDB
EXTENDS Naturals, Sequences, Bags, FiniteSets

CONSTANT MaxNodes, MaxEvents

VARIABLES 
    nodes,          \* Множество активных узлов
    edges,          \* Множество рёбер
    events,         \* История событий (последовательность)
    states,         \* Состояния объектов с версионированием
    clock           \* Логические часы

Init ==
    /\ nodes = {}
    /\ edges = {}
    /\ events = <<>>
    /\ states = [n \in {} |-> <<>>]
    /\ clock = 0

\* Инвариант: все рёбра указывают на существующие узлы
RefIntegrity ==
    \A e \in edges: e.from \in nodes /\ e.to \in nodes

\* Инвариант: версии монотонно возрастают
MonotonicVersions ==
    \A n \in nodes:
        LET hist == states[n] IN
        IF Len(hist) > 1 THEN 
            \A i \in 1..Len(hist)-1: hist[i].version < hist[i+1].version
        ELSE TRUE

\* Инвариант: Timestampy возрастают монотонно
TemporalOrdering ==
    \A n \in nodes:
        LET hist == states[n] IN
        IF Len(hist) > 1 THEN 
            \A i \in 1..Len(hist)-1: hist[i].timestamp < hist[i+1].timestamp
        ELSE TRUE

\* Инвариант: детерминированность replay
ReplayDeterminism ==
    TRUE  \* Проверяется в коде, не в TLA+

Invariants == 
    /\ RefIntegrity
    /\ MonotonicVersions
    /\ TemporalOrdering
    /\ Cardinality(nodes) <= MaxNodes
    /\ Len(events) <= MaxEvents
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_referential_integrity() {
        let mut graph = GraphStore::new();
        graph.add_node(Node::new("n1".to_string(), vec![]));
        graph.add_edge(Edge::new(
            "e1".to_string(),
            "n1".to_string(),
            "n_fake".to_string(),
            "TEST".to_string(),
        ));

        let result = ConsistencyVerifier::verify_referential_integrity(&graph);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_state_tracer() {
        let mut tracer = StateTracer::new();
        tracer.trace("CREATE", "n1", "{}", "{name: Alice}");
        tracer.trace("UPDATE", "n1", "{name: Alice}", "{name: Alice, age: 28}");

        assert_eq!(tracer.get_trace_log().len(), 2);
    }
}
