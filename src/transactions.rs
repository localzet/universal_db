/// Модуль Phase 5: Транзакции и разрешение конфликтов
///
/// Включает реализацию:
/// - Базовый механизм транзакций (группировка событий)
/// - Стратегии разрешения конфликтов (LWW, CRDT, custom merge)
/// - Откат транзакций
/// - История транзакций

use crate::models::*;
use crate::store::StateStore;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Стратегия разрешения конфликтов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolutionStrategy {
    /// Last Write Wins - победит последний по времени
    LastWriteWins,
    /// First Write Wins - победит первый и игнорируются изменения
    FirstWriteWins,
    /// Manual merge - требует пользовательской логики
    Manual,
    /// CRDT-like - объединить значения
    Merge,
}

/// Транзакция - группа событий с тем же timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub events: Vec<Event>,
    pub timestamp: DateTime<Utc>,
    pub status: TransactionStatus,
    pub conflict_strategy: ConflictResolutionStrategy,
}

/// Статус транзакции
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TransactionStatus {
    Pending,
    Committed,
    Aborted,
    Rolled,
}

/// Конфликт между событиями
#[derive(Debug, Clone)]
pub struct Conflict {
    pub event_id_1: String,
    pub event_id_2: String,
    pub target_id: String,
    pub conflict_type: ConflictType,
}

/// Тип конфликта
#[derive(Debug, Clone)]
pub enum ConflictType {
    /// Оба события изменяют одно свойство
    PropertyConflict,
    /// Одно событие удаляет, другое обновляет
    DeleteUpdateConflict,
    /// Циклические зависимости
    CyclicDependency,
}

/// Менеджер транзакций
pub struct TransactionManager {
    transactions: HashMap<String, Transaction>,
    committed_transactions: Vec<Transaction>,
    conflicts: Vec<Conflict>,
}

impl TransactionManager {
    pub fn new() -> Self {
        TransactionManager {
            transactions: HashMap::new(),
            committed_transactions: Vec::new(),
            conflicts: Vec::new(),
        }
    }

    /// Создать новую транзакцию
    pub fn create_transaction(
        &mut self,
        id: String,
        strategy: ConflictResolutionStrategy,
    ) -> Transaction {
        let tx = Transaction {
            id: id.clone(),
            events: Vec::new(),
            timestamp: Utc::now(),
            status: TransactionStatus::Pending,
            conflict_strategy: strategy,
        };
        self.transactions.insert(id, tx.clone());
        tx
    }

    /// Добавить событие в транзакцию
    pub fn add_event_to_transaction(
        &mut self,
        transaction_id: &str,
        event: Event,
    ) -> Result<(), String> {
        if let Some(tx) = self.transactions.get_mut(transaction_id) {
            if tx.status != TransactionStatus::Pending {
                return Err("Транзакция уже завершена".to_string());
            }
            tx.events.push(event);
            Ok(())
        } else {
            Err("Транзакция не найдена".to_string())
        }
    }

    /// Зафиксировать транзакцию (подтвердить все события)
    pub fn commit_transaction(
        &mut self,
        transaction_id: &str,
    ) -> Result<(), String> {
        if let Some(mut tx) = self.transactions.remove(transaction_id) {
            // Проверить конфликты
            let conflicts = self.detect_conflicts(&tx);
            if !conflicts.is_empty() {
                self.conflicts.extend(conflicts);
                // Попытаться разрешить конфликты в соответствии со стратегией
                match tx.conflict_strategy {
                    ConflictResolutionStrategy::LastWriteWins => {
                        // LWW автоматически разрешает - позже событие побеждает
                    }
                    ConflictResolutionStrategy::FirstWriteWins => {
                        // Отменить конфликтующие события
                        tx.events.retain(|e| !self.conflicts.iter().any(|c| c.event_id_2 == e.id));
                    }
                    ConflictResolutionStrategy::Manual => {
                        return Err("Требуется ручное разрешение конфликтов".to_string());
                    }
                    ConflictResolutionStrategy::Merge => {
                        // Попытаться слить конфликтующие события
                        self.merge_conflicting_events(&mut tx);
                    }
                }
            }

            tx.status = TransactionStatus::Committed;
            self.committed_transactions.push(tx);
            Ok(())
        } else {
            Err("Транзакция не найдена".to_string())
        }
    }

    /// Откатить транзакцию (отменить все события)
    pub fn abort_transaction(&mut self, transaction_id: &str) -> Result<(), String> {
        if let Some(mut tx) = self.transactions.remove(transaction_id) {
            tx.status = TransactionStatus::Aborted;
            self.committed_transactions.push(tx);
            Ok(())
        } else {
            Err("Транзакция не найдена".to_string())
        }
    }

    /// Откатить зафиксированную транзакцию на одну версию назад
    pub fn rollback_transaction(
        &mut self,
        transaction_id: &str,
    ) -> Result<(), String> {
        // Найти транзакцию в истории
        if let Some(pos) = self.committed_transactions.iter().position(|tx| tx.id == transaction_id) {
            let mut tx = self.committed_transactions.remove(pos);
            tx.status = TransactionStatus::Rolled;
            self.committed_transactions.push(tx);
            Ok(())
        } else {
            Err("Транзакция не найдена в истории".to_string())
        }
    }

    /// Обнаружить конфликты в транзакции
    fn detect_conflicts(&self, tx: &Transaction) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        // Проверить, меняет ли каждое событие один и тот же объект
        for i in 0..tx.events.len() {
            for j in (i + 1)..tx.events.len() {
                let e1 = &tx.events[i];
                let e2 = &tx.events[j];

                if e1.target_id == e2.target_id {
                    // Обнаружен конфликт - оба события меняют один объект
                    conflicts.push(Conflict {
                        event_id_1: e1.id.clone(),
                        event_id_2: e2.id.clone(),
                        target_id: e1.target_id.clone(),
                        conflict_type: ConflictType::PropertyConflict,
                    });
                }
            }
        }

        conflicts
    }

    /// Слить конфликтующие события (simple merge)
    fn merge_conflicting_events(&self, _tx: &mut Transaction) {
        // Simplified merge: just keep both changes
        // In real implementation, would do smarter merging
    }

    /// Получить статус транзакции
    pub fn get_transaction_status(&self, transaction_id: &str) -> Option<TransactionStatus> {
        self.transactions
            .get(transaction_id)
            .map(|tx| tx.status.clone())
            .or_else(|| {
                self.committed_transactions
                    .iter()
                    .find(|tx| tx.id == transaction_id)
                    .map(|tx| tx.status.clone())
            })
    }

    /// Получить историю транзакций
    pub fn get_history(&self) -> Vec<Transaction> {
        self.committed_transactions.clone()
    }

    /// Получить все конфликты
    pub fn get_conflicts(&self) -> Vec<Conflict> {
        self.conflicts.clone()
    }

    /// Применить события из транзакции к состоянию
    pub fn apply_transaction(
        &self,
        transaction: &Transaction,
        state_store: &mut StateStore,
    ) -> Result<(), String> {
        if transaction.status != TransactionStatus::Committed {
            return Err("Только зафиксированные транзакции можно применять".to_string());
        }

        for event in &transaction.events {
            state_store.apply_event(event);
        }

        Ok(())
    }

    /// Получить количество ожидающих транзакций
    pub fn pending_count(&self) -> usize {
        self.transactions.len()
    }

    /// Получить количество зафиксированных транзакций
    pub fn committed_count(&self) -> usize {
        self.committed_transactions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_create_and_commit() {
        let mut manager = TransactionManager::new();

        let tx = manager.create_transaction(
            "tx1".to_string(),
            ConflictResolutionStrategy::LastWriteWins,
        );

        assert_eq!(tx.status, TransactionStatus::Pending);

        let event = Event::new(
            "e1".to_string(),
            "n1".to_string(),
            EventType::CreateNode,
            serde_json::json!({"name": "Alice"}),
        );

        manager.add_event_to_transaction("tx1", event).unwrap();
        manager.commit_transaction("tx1").unwrap();

        assert_eq!(manager.committed_count(), 1);
    }

    #[test]
    fn test_transaction_abort() {
        let mut manager = TransactionManager::new();

        manager.create_transaction("tx1".to_string(), ConflictResolutionStrategy::LastWriteWins);

        manager.abort_transaction("tx1").unwrap();

        assert_eq!(manager.pending_count(), 0);
        assert_eq!(manager.committed_count(), 1);
    }

    #[test]
    fn test_conflict_detection() {
        let mut manager = TransactionManager::new();

        let _tx = manager.create_transaction(
            "tx1".to_string(),
            ConflictResolutionStrategy::LastWriteWins,
        );

        // Оба события меняют один объект
        let event1 = Event::new(
            "e1".to_string(),
            "n1".to_string(),
            EventType::UpdateNode,
            serde_json::json!({"age": 28}),
        );

        let event2 = Event::new(
            "e2".to_string(),
            "n1".to_string(),
            EventType::UpdateNode,
            serde_json::json!({"age": 30}),
        );

        manager.add_event_to_transaction("tx1", event1).unwrap();
        manager.add_event_to_transaction("tx1", event2).unwrap();

        let conflicts = manager.detect_conflicts(&manager.transactions.get("tx1").unwrap());
        assert_eq!(conflicts.len(), 1);
    }
}
