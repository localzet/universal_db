/// Модуль Phase 2: Графовые алгоритмы
/// 
/// Включает реализацию:
/// - BFS (поиск в ширину)
/// - DFS (поиск в глубину)
/// - Поиск кратчайшего пути (Dijkstra)
/// - Временные запросы (путь с фильтром по времени)
/// - Анализ достижимости

use crate::store::{GraphStore, StateStore};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet, VecDeque};

/// Результат поиска в ширину
#[derive(Debug, Clone)]
pub struct BfsResult {
    pub visited: Vec<String>,
    pub distances: HashMap<String, usize>,
    pub parent: HashMap<String, String>,
}

/// Результат поиска пути
#[derive(Debug, Clone)]
pub struct PathResult {
    pub path: Vec<String>,
    pub distance: usize,
    pub exists: bool,
}

/// Результат анализа графа
#[derive(Debug, Clone)]
pub struct GraphAnalysis {
    pub connected_components: Vec<Vec<String>>,
    pub node_count: usize,
    pub edge_count: usize,
}

/// Поиск в ширину (BFS) с временным фильтром
pub fn bfs(
    graph: &GraphStore,
    start_node: &str,
    max_depth: Option<usize>,
) -> BfsResult {
    let mut visited = Vec::new();
    let mut distances = HashMap::new();
    let mut parent = HashMap::new();
    let mut queue = VecDeque::new();

    queue.push_back((start_node.to_string(), 0));
    distances.insert(start_node.to_string(), 0);

    while let Some((current, depth)) = queue.pop_front() {
        if visited.contains(&current) {
            continue;
        }

        visited.push(current.clone());

        if let Some(max) = max_depth {
            if depth >= max {
                continue;
            }
        }

        // Получить соседей текущего узла
        for edge in graph.get_neighbors(&current) {
            let neighbor = edge.to.clone();
            if !visited.contains(&neighbor) && !distances.contains_key(&neighbor) {
                distances.insert(neighbor.clone(), depth + 1);
                parent.insert(neighbor.clone(), current.clone());
                queue.push_back((neighbor, depth + 1));
            }
        }
    }

    BfsResult {
        visited,
        distances,
        parent,
    }
}

/// Поиск в глубину (DFS)
pub fn dfs(
    graph: &GraphStore,
    start_node: &str,
) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    dfs_recursive(graph, start_node, &mut visited, &mut result);
    result
}

/// Вспомогательная рекурсивная функция для DFS
fn dfs_recursive(
    graph: &GraphStore,
    node: &str,
    visited: &mut HashSet<String>,
    result: &mut Vec<String>,
) {
    visited.insert(node.to_string());
    result.push(node.to_string());

    for edge in graph.get_neighbors(node) {
        if !visited.contains(&edge.to) {
            dfs_recursive(graph, &edge.to, visited, result);
        }
    }
}

/// Поиск кратчайшего пути (BFS-based)
pub fn find_shortest_path(
    graph: &GraphStore,
    start: &str,
    end: &str,
) -> PathResult {
    let bfs_result = bfs(graph, start, None);

    if !bfs_result.distances.contains_key(end) {
        return PathResult {
            path: vec![],
            distance: 0,
            exists: false,
        };
    }

    let mut path = vec![end.to_string()];
    let mut current = end.to_string();

    while current != start {
        if let Some(parent) = bfs_result.parent.get(&current) {
            path.push(parent.clone());
            current = parent.clone();
        } else {
            break;
        }
    }

    path.reverse();

    PathResult {
        path: path.clone(),
        distance: bfs_result.distances.get(end).copied().unwrap_or(0),
        exists: true,
    }
}

/// Поиск всех путей из стартовой точки
pub fn find_all_reachable(
    graph: &GraphStore,
    start_node: &str,
) -> HashMap<String, Vec<String>> {
    let mut result = HashMap::new();

    for node in graph.nodes.keys() {
        if let PathResult { exists: true, path, .. } = find_shortest_path(graph, start_node, node) {
            result.insert(node.clone(), path);
        }
    }

    result
}

/// Анализ компонент связности (для ориентированного графа)
pub fn analyze_connectivity(graph: &GraphStore) -> GraphAnalysis {
    let mut connected_components = Vec::new();
    let mut visited = HashSet::new();

    for node_id in graph.nodes.keys() {
        if !visited.contains(node_id) {
            let component = dfs(graph, node_id);
            for n in &component {
                visited.insert(n.to_string());
            }
            connected_components.push(component);
        }
    }

    GraphAnalysis {
        connected_components,
        node_count: graph.nodes.len(),
        edge_count: graph.edges.len(),
    }
}

/// Поиск соседей с фильтром по типу края
pub fn find_neighbors_by_type(
    graph: &GraphStore,
    node_id: &str,
    edge_type: &str,
) -> Vec<String> {
    graph
        .get_neighbors(node_id)
        .into_iter()
        .filter(|e| e.edge_type == edge_type)
        .map(|e| e.to.clone())
        .collect()
}

/// Поиск циклов в графе (DFS-based)
pub fn find_cycles(graph: &GraphStore) -> Vec<Vec<String>> {
    let mut cycles = Vec::new();
    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();
    let mut path = Vec::new();

    for node in graph.nodes.keys() {
        if !visited.contains(node) {
            find_cycles_dfs(
                graph,
                node,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    cycles
}

/// Вспомогательная функция для поиска циклов
fn find_cycles_dfs(
    graph: &GraphStore,
    node: &str,
    visited: &mut HashSet<String>,
    rec_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<Vec<String>>,
) {
    visited.insert(node.to_string());
    rec_stack.insert(node.to_string());
    path.push(node.to_string());

    for edge in graph.get_neighbors(node) {
        if rec_stack.contains(&edge.to) {
            // Найден цикл
            if let Some(start_idx) = path.iter().position(|n| n == &edge.to) {
                let cycle = path[start_idx..].to_vec();
                cycles.push(cycle);
            }
        } else if !visited.contains(&edge.to) {
            find_cycles_dfs(graph, &edge.to, visited, rec_stack, path, cycles);
        }
    }

    path.pop();
    rec_stack.remove(node);
}

/// Временной фильтр для запросов (найти соседей, которые были активны в период времени)
pub fn find_neighbors_at_time(
    graph: &GraphStore,
    state_store: &StateStore,
    node_id: &str,
    timestamp: &DateTime<Utc>,
) -> Vec<String> {
    graph
        .get_neighbors(node_id)
        .into_iter()
        .filter_map(|edge| {
            // Проверить существует ли состояние ребра в эту дату
            if state_store.state_at(&edge.id, timestamp).is_some() {
                Some(edge.to.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Анализ степеней узлов (in/out)
pub fn calculate_node_degrees(graph: &GraphStore) -> HashMap<String, (usize, usize)> {
    let mut degrees: HashMap<String, (usize, usize)> = HashMap::new();

    // Инициализируем все узлы
    for node_id in graph.nodes.keys() {
        degrees.insert(node_id.clone(), (0, 0));
    }

    // Подсчитываем out-degree
    for edge in graph.edges.values() {
        if let Some((_, out)) = degrees.get_mut(&edge.from) {
            *out += 1;
        }

        // Подсчитываем in-degree
        if let Some((in_deg, _)) = degrees.get_mut(&edge.to) {
            *in_deg += 1;
        }
    }

    degrees
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Node, Edge};

    #[test]
    fn test_bfs_basic() {
        let mut graph = GraphStore::new();

        // Создаём граф: n1 -> n2 -> n3
        graph.add_node(Node::new("n1".to_string(), vec![]));
        graph.add_node(Node::new("n2".to_string(), vec![]));
        graph.add_node(Node::new("n3".to_string(), vec![]));

        graph.add_edge(Edge::new(
            "e1".to_string(),
            "n1".to_string(),
            "n2".to_string(),
            "NEXT".to_string(),
        ));
        graph.add_edge(Edge::new(
            "e2".to_string(),
            "n2".to_string(),
            "n3".to_string(),
            "NEXT".to_string(),
        ));

        let result = bfs(&graph, "n1", None);
        assert_eq!(result.visited.len(), 3);
        assert_eq!(*result.distances.get("n1").unwrap(), 0);
        assert_eq!(*result.distances.get("n2").unwrap(), 1);
        assert_eq!(*result.distances.get("n3").unwrap(), 2);
    }

    #[test]
    fn test_dfs_basic() {
        let mut graph = GraphStore::new();
        graph.add_node(Node::new("n1".to_string(), vec![]));
        graph.add_node(Node::new("n2".to_string(), vec![]));

        graph.add_edge(Edge::new(
            "e1".to_string(),
            "n1".to_string(),
            "n2".to_string(),
            "NEXT".to_string(),
        ));

        let result = dfs(&graph, "n1");
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_shortest_path() {
        let mut graph = GraphStore::new();
        graph.add_node(Node::new("n1".to_string(), vec![]));
        graph.add_node(Node::new("n2".to_string(), vec![]));
        graph.add_node(Node::new("n3".to_string(), vec![]));

        graph.add_edge(Edge::new(
            "e1".to_string(),
            "n1".to_string(),
            "n2".to_string(),
            "NEXT".to_string(),
        ));
        graph.add_edge(Edge::new(
            "e2".to_string(),
            "n2".to_string(),
            "n3".to_string(),
            "NEXT".to_string(),
        ));

        let result = find_shortest_path(&graph, "n1", "n3");
        assert!(result.exists);
        assert_eq!(result.distance, 2);
        assert_eq!(result.path, vec!["n1".to_string(), "n2".to_string(), "n3".to_string()]);
    }
}
