use crate::models::*;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Хранилище графовых объектов (Node/Edge)
pub struct GraphStore {
    pub nodes: HashMap<String, Node>,
    pub edges: HashMap<String, Edge>,
    pub adjacency: HashMap<String, Vec<String>>, // Node id -> Edge ids (outgoing)
}

impl GraphStore {
    pub fn new() -> Self {
        GraphStore {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            adjacency: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: Edge) {
        self.adjacency
            .entry(edge.from.clone())
            .or_insert_with(Vec::new)
            .push(edge.id.clone());
        self.edges.insert(edge.id.clone(), edge);
    }

    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    pub fn get_edge(&self, id: &str) -> Option<&Edge> {
        self.edges.get(id)
    }

    pub fn get_neighbors(&self, node_id: &str) -> Vec<&Edge> {
        self.adjacency
            .get(node_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|eid| self.edges.get(eid))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn delete_node(&mut self, id: &str) {
        self.nodes.remove(id);
        self.adjacency.remove(id);
        // Remove edges connected to this node
        self.edges.retain(|_, edge| edge.from != id && edge.to != id);
    }
}

/// Хранилище событий (Event Log) — append-only
pub struct EventStore {
    pub events: Vec<Event>,
}

impl EventStore {
    pub fn new() -> Self {
        EventStore {
            events: Vec::new(),
        }
    }

    pub fn append_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub fn get_events_for_target(&self, target_id: &str) -> Vec<&Event> {
        self.events
            .iter()
            .filter(|e| e.target_id == target_id)
            .collect()
    }

    pub fn get_events_after(&self, timestamp: &DateTime<Utc>) -> Vec<&Event> {
        self.events.iter().filter(|e| e.timestamp > *timestamp).collect()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

/// Хранилище состояний с версионированием
pub struct StateStore {
    // target_id -> версии состояний упорядочены по времени
    pub states: HashMap<String, Vec<State>>,
}

impl StateStore {
    pub fn new() -> Self {
        StateStore {
            states: HashMap::new(),
        }
    }

    /// Применить событие: создать новое состояние на основе последнего
    pub fn apply_event(&mut self, event: &Event) {
        let new_props = match self.states.get(&event.target_id) {
            Some(history) => {
                let last_state = history.last().unwrap();
                let mut props = last_state.properties.clone();
                // Merge payload into properties
                if let serde_json::Value::Object(map) = &event.payload {
                    for (k, v) in map.iter() {
                        props.insert(k.clone(), v.clone());
                    }
                }
                props
            }
            None => {
                let mut props = HashMap::new();
                if let serde_json::Value::Object(map) = &event.payload {
                    for (k, v) in map.iter() {
                        props.insert(k.clone(), v.clone());
                    }
                }
                props
            }
        };

        let version = self
            .states
            .get(&event.target_id)
            .map(|v| v.len() as u64 + 1)
            .unwrap_or(1);

        let state = State::new(
            event.target_id.clone(),
            version,
            new_props,
            event.timestamp,
        );

        self.states
            .entry(event.target_id.clone())
            .or_insert_with(Vec::new)
            .push(state);
    }

    /// Получить состояние объекта на определённый момент времени
    pub fn state_at(
        &self,
        target_id: &str,
        timestamp: &DateTime<Utc>,
    ) -> Option<State> {
        self.states
            .get(target_id)
            .and_then(|history| {
                history
                    .iter()
                    .rev()
                    .find(|s| s.timestamp <= *timestamp)
                    .cloned()
            })
    }

    /// Получить последнее (актуальное) состояние объекта
    pub fn current_state(&self, target_id: &str) -> Option<State> {
        self.states
            .get(target_id)
            .and_then(|history| history.last().cloned())
    }

    /// Получить полную историю объекта
    pub fn history(&self, target_id: &str) -> Option<Vec<State>> {
        self.states.get(target_id).cloned()
    }

    /// Compute diff между двумя версиями
    pub fn diff(
        &self,
        target_id: &str,
        version1: u64,
        version2: u64,
    ) -> Option<(State, State)> {
        let history = self.states.get(target_id)?;
        let state1 = history.iter().find(|s| s.version == version1)?.clone();
        let state2 = history.iter().find(|s| s.version == version2)?.clone();
        Some((state1, state2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_store() {
        let mut store = GraphStore::new();
        let node = Node::new("n1".to_string(), vec!["Person".to_string()]);
        store.add_node(node);

        assert!(store.get_node("n1").is_some());
    }

    #[test]
    fn test_event_store() {
        let mut store = EventStore::new();
        let event = Event::new(
            "e1".to_string(),
            "n1".to_string(),
            EventType::CreateNode,
            serde_json::json!({"name": "Alice"}),
        );
        store.append_event(event);

        assert_eq!(store.len(), 1);
    }
}
