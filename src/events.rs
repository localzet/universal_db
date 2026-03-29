use crate::models::*;
use crate::store::{EventStore, StateStore};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Проиграть события и построить актуальное состояние
pub fn replay_events(events: &[Event], state_store: &mut StateStore) {
    for event in events {
        state_store.apply_event(event);
    }
}

/// Проиграть события до определённого момента времени
pub fn replay_events_until(
    events: &[Event],
    state_store: &mut StateStore,
    until_timestamp: &DateTime<Utc>,
) {
    for event in events {
        if event.timestamp <= *until_timestamp {
            state_store.apply_event(event);
        } else {
            break;
        }
    }
}

/// Восстановить состояние из Event Store
pub fn rebuild_state(event_store: &EventStore, state_store: &mut StateStore) {
    let events = &event_store.events.clone();
    replay_events(events, state_store);
}

/// Snapshot for point-in-time recovery
pub fn create_snapshot(
    state_store: &StateStore,
    timestamp: DateTime<Utc>,
) -> Snapshot {
    let mut nodes = HashMap::new();
    let mut edges = HashMap::new();

    for (target_id, history) in &state_store.states {
        if let Some(state) = history.iter().rev().find(|s| s.timestamp <= timestamp) {
            if target_id.starts_with("n") {
                nodes.insert(target_id.clone(), state.clone());
            } else if target_id.starts_with("e") {
                edges.insert(target_id.clone(), state.clone());
            }
        }
    }

    Snapshot {
        id: format!("snapshot_{}", uuid::Uuid::new_v4()),
        timestamp,
        nodes,
        edges,
    }
}

/// Откатить состояние на одну версию назад
pub fn rollback(
    state_store: &StateStore,
    target_id: &str,
) -> Option<State> {
    state_store
        .history(target_id)
        .and_then(|mut history| {
            if history.len() > 1 {
                history.pop();
                history.last().cloned()
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_replay() {
        let mut state_store = StateStore::new();

        let event1 = Event::new(
            "e1".to_string(),
            "n1".to_string(),
            EventType::CreateNode,
            serde_json::json!({"name": "Alice"}),
        );

        let event2 = Event::new(
            "e2".to_string(),
            "n1".to_string(),
            EventType::UpdateNode,
            serde_json::json!({"age": 22}),
        );

        let events = vec![event1, event2];
        replay_events(&events, &mut state_store);

        let current = state_store.current_state("n1");
        assert!(current.is_some());
        let state = current.unwrap();
        assert_eq!(state.version, 2);
        assert_eq!(state.properties.get("name").and_then(|v| v.as_str()), Some("Alice"));
        assert_eq!(state.properties.get("age").and_then(|v| v.as_i64()), Some(22));
    }

    #[test]
    fn test_time_travel() {
        let mut state_store = StateStore::new();
        let now = Utc::now();

        let event1 = Event {
            id: "e1".to_string(),
            target_id: "n1".to_string(),
            event_type: EventType::CreateNode,
            payload: serde_json::json!({"name": "Alice"}),
            timestamp: now,
        };

        let event2 = Event {
            id: "e2".to_string(),
            target_id: "n1".to_string(),
            event_type: EventType::UpdateNode,
            payload: serde_json::json!({"age": 22}),
            timestamp: now + chrono::Duration::seconds(10),
        };

        replay_events(&vec![event1, event2], &mut state_store);

        // Check state between two events
        let old_state = state_store.state_at("n1", &(now + chrono::Duration::seconds(5)));
        assert!(old_state.is_some());
        assert_eq!(old_state.unwrap().version, 1);
    }
}

use uuid;
