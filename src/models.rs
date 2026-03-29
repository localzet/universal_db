use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub labels: Vec<String>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl Node {
    pub fn new(id: String, labels: Vec<String>) -> Self {
        Node {
            id,
            labels,
            properties: HashMap::new(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub edge_type: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl Edge {
    pub fn new(id: String, from: String, to: String, edge_type: String) -> Self {
        Edge {
            id,
            from,
            to,
            edge_type,
            properties: HashMap::new(),
            created_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    CreateNode,
    UpdateNode,
    DeleteNode,
    CreateEdge,
    UpdateEdge,
    DeleteEdge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub target_id: String,      // Node/Edge id
    pub event_type: EventType,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

impl Event {
    pub fn new(
        id: String,
        target_id: String,
        event_type: EventType,
        payload: serde_json::Value,
    ) -> Self {
        Event {
            id,
            target_id,
            event_type,
            payload,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub target_id: String,
    pub version: u64,
    pub properties: HashMap<String, serde_json::Value>,
    pub timestamp: DateTime<Utc>,
}

impl State {
    pub fn new(
        target_id: String,
        version: u64,
        properties: HashMap<String, serde_json::Value>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        State {
            target_id,
            version,
            properties,
            timestamp,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub nodes: HashMap<String, State>,
    pub edges: HashMap<String, State>,
}
