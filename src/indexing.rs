/// Модуль Phase 3: Индексирование и оптимизация запросов
///
/// Включает реализацию:
/// - B-tree индексы для временных запросов
/// - Индексы по типам рёбер
/// - Индексы по значениям свойств
/// - Кэширование результатов запросов

use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

/// Индекс на основе B-tree для временных диапазонов
#[derive(Debug, Clone)]
pub struct TimeRangeIndex {
    /// Отображение от timestamp (в наносекундах) к ID объектов
    ranges: BTreeMap<i64, Vec<String>>,
}

impl TimeRangeIndex {
    pub fn new() -> Self {
        TimeRangeIndex {
            ranges: BTreeMap::new(),
        }
    }

    /// Добавить объект в индекс по времени
    pub fn add(&mut self, timestamp_nanos: i64, object_id: String) {
        self.ranges
            .entry(timestamp_nanos)
            .or_insert_with(Vec::new)
            .push(object_id);
    }

    /// Найти объекты в диапазоне времени
    pub fn range_query(&self, start: i64, end: i64) -> Vec<String> {
        let mut result = Vec::new();
        for (_, ids) in self.ranges.range(start..=end) {
            result.extend(ids.clone());
        }
        result
    }

    /// Найти объекты после определённого времени
    pub fn after(&self, timestamp_nanos: i64) -> Vec<String> {
        let mut result = Vec::new();
        for (_, ids) in self.ranges.range(timestamp_nanos..) {
            result.extend(ids.clone());
        }
        result
    }

    /// Найти объекты до определённого времени
    pub fn before(&self, timestamp_nanos: i64) -> Vec<String> {
        let mut result = Vec::new();
        for (_, ids) in self.ranges.range(..=timestamp_nanos) {
            result.extend(ids.clone());
        }
        result
    }
}

/// Индекс по типам (для рёбер: KNOWS, CREATED_BY, OWNS и т.д.)
#[derive(Debug, Clone)]
pub struct TypeIndex {
    /// Отображение от типа к списку объектов
    type_map: HashMap<String, Vec<String>>,
}

impl TypeIndex {
    pub fn new() -> Self {
        TypeIndex {
            type_map: HashMap::new(),
        }
    }

    /// Добавить объект в индекс по типу
    pub fn add(&mut self, type_name: String, object_id: String) {
        self.type_map
            .entry(type_name)
            .or_insert_with(Vec::new)
            .push(object_id);
    }

    /// Найти все объекты определённого типа
    pub fn get_by_type(&self, type_name: &str) -> Vec<String> {
        self.type_map
            .get(type_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Получить статистику по типам
    pub fn get_stats(&self) -> HashMap<String, usize> {
        self.type_map
            .iter()
            .map(|(k, v)| (k.clone(), v.len()))
            .collect()
    }
}

/// Индекс по свойствам (для быстрого поиска по значениям)
#[derive(Debug, Clone)]
pub struct PropertyIndex {
    /// Отображение от (свойство, значение) к списку объектов
    property_map: HashMap<(String, String), Vec<String>>,
}

impl PropertyIndex {
    pub fn new() -> Self {
        PropertyIndex {
            property_map: HashMap::new(),
        }
    }

    /// Добавить объект в индекс по свойству
    pub fn add(&mut self, property_name: String, value: Value, object_id: String) {
        let value_str = value.to_string();
        let key = (property_name, value_str);
        self.property_map
            .entry(key)
            .or_insert_with(Vec::new)
            .push(object_id);
    }

    /// Найти объекты с определённым значением свойства
    pub fn get_by_property(&self, property: &str, value: &str) -> Vec<String> {
        let key = (property.to_string(), value.to_string());
        self.property_map.get(&key).cloned().unwrap_or_default()
    }

    /// Найти объекты, имеющие определённое свойство (любого значения)
    pub fn get_by_property_name(&self, property: &str) -> Vec<String> {
        let mut result = Vec::new();
        for ((prop, _), ids) in &self.property_map {
            if prop == property {
                result.extend(ids.clone());
            }
        }
        result
    }
}

/// Индекс префиксов для строковых значений
#[derive(Debug, Clone)]
pub struct PrefixIndex {
    /// Отображение от префикса к объектам
    prefix_map: HashMap<String, Vec<String>>,
}

impl PrefixIndex {
    pub fn new() -> Self {
        PrefixIndex {
            prefix_map: HashMap::new(),
        }
    }

    /// Добавить объект с генерацией префиксов
    pub fn add(&mut self, text: &str, object_id: String) {
        let text_lower = text.to_lowercase();
        // Генерируем все префиксы до длины 4
        for i in 1..=text_lower.len().min(4) {
            let prefix = &text_lower[..i];
            self.prefix_map
                .entry(prefix.to_string())
                .or_insert_with(Vec::new)
                .push(object_id.clone());
        }
    }

    /// Найти объекты по префиксу
    pub fn search_prefix(&self, prefix: &str) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        self.prefix_map
            .get(&prefix_lower)
            .cloned()
            .unwrap_or_default()
    }
}

/// Кэш для результатов запросов
#[derive(Debug, Clone)]
pub struct QueryCache {
    cache: HashMap<String, (Vec<String>, chrono::DateTime<chrono::Utc>)>,
    ttl_secs: u64,
}

impl QueryCache {
    pub fn new(ttl_secs: u64) -> Self {
        QueryCache {
            cache: HashMap::new(),
            ttl_secs,
        }
    }

    /// Сохранить результат запроса в кэш
    pub fn put(&mut self, key: String, value: Vec<String>) {
        self.cache.insert(key, (value, chrono::Utc::now()));
    }

    /// Получить результат из кэша (если не истёк TTL)
    pub fn get(&self, key: &str) -> Option<Vec<String>> {
        if let Some((value, timestamp)) = self.cache.get(key) {
            let elapsed = chrono::Utc::now()
                .signed_duration_since(*timestamp)
                .num_seconds() as u64;

            if elapsed < self.ttl_secs {
                return Some(value.clone());
            }
        }
        None
    }

    /// Очистить кэш
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Получить размер кэша
    pub fn size(&self) -> usize {
        self.cache.len()
    }
}

/// Составной индекс для оптимизации запросов
pub struct CompositeIndex {
    pub time_index: TimeRangeIndex,
    pub type_index: TypeIndex,
    pub property_index: PropertyIndex,
    pub prefix_index: PrefixIndex,
    pub query_cache: QueryCache,
}

impl CompositeIndex {
    pub fn new() -> Self {
        CompositeIndex {
            time_index: TimeRangeIndex::new(),
            type_index: TypeIndex::new(),
            property_index: PropertyIndex::new(),
            prefix_index: PrefixIndex::new(),
            query_cache: QueryCache::new(60), // 60 сек TTL
        }
    }

    /// Получить статистику индексов
    pub fn get_stats(&self) -> IndexStats {
        IndexStats {
            time_index_entries: self.time_index.ranges.len(),
            type_index_entries: self.type_index.type_map.len(),
            property_index_entries: self.property_index.property_map.len(),
            prefix_index_entries: self.prefix_index.prefix_map.len(),
            cache_size: self.query_cache.size(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexStats {
    pub time_index_entries: usize,
    pub type_index_entries: usize,
    pub property_index_entries: usize,
    pub prefix_index_entries: usize,
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_range_index() {
        let mut index = TimeRangeIndex::new();

        index.add(1000, "obj1".to_string());
        index.add(2000, "obj2".to_string());
        index.add(3000, "obj3".to_string());

        let result = index.range_query(1500, 2500);
        assert_eq!(result.len(), 1);
        assert!(result.contains(&"obj2".to_string()));
    }

    #[test]
    fn test_type_index() {
        let mut index = TypeIndex::new();
        index.add("KNOWS".to_string(), "edge1".to_string());
        index.add("KNOWS".to_string(), "edge2".to_string());
        index.add("CREATED".to_string(), "edge3".to_string());

        let knows = index.get_by_type("KNOWS");
        assert_eq!(knows.len(), 2);

        let stats = index.get_stats();
        assert_eq!(stats.len(), 2);
    }

    #[test]
    fn test_property_index() {
        let mut index = PropertyIndex::new();
        index.add("name".to_string(), Value::String("Alice".to_string()), "n1".to_string());
        index.add("name".to_string(), Value::String("Bob".to_string()), "n2".to_string());

        let alice = index.get_by_property("name", "\"Alice\"");
        assert_eq!(alice.len(), 1);
    }

    #[test]
    fn test_query_cache() {
        let mut cache = QueryCache::new(60);
        cache.put("query1".to_string(), vec!["result1".to_string()]);

        let result = cache.get("query1");
        assert_eq!(result, Some(vec!["result1".to_string()]));
    }
}
