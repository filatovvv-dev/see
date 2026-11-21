//! Производство: рецепты, очередь заданий и применение результатов.
//!
//! Все коллекции здесь детерминированы: используются только `Vec` и `BTreeMap`
//! (см. AGENTS.md, раздел «Главное правило детерминизма»). Время задаётся
//! внутренним счётчиком тиков симуляции, а не системными часами.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::entities::{Cargo, ItemId};

/// Рецепт производства: какие предметы потребляются и производятся за один запуск.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    /// Идентификатор рецепта.
    pub id: u32,
    /// Требуемые входы: `предмет -> количество` на один запуск.
    pub inputs: BTreeMap<ItemId, u64>,
    /// Получаемые выходы: `предмет -> количество` на один запуск.
    pub outputs: BTreeMap<ItemId, u64>,
    /// Длительность одного задания в тиках симуляции.
    pub time_ticks: u64,
}

/// Завершённое производственное задание.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletedBatch {
    /// Идентификатор выполненного рецепта.
    pub recipe_id: u32,
    /// Количество запусков рецепта в задании.
    pub quantity: u64,
    /// Итоговые выходы с учётом `quantity` (`recipe.outputs[i] * quantity`).
    pub outputs: BTreeMap<ItemId, u64>,
}

/// Задание в очереди производства.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Job {
    recipe: Recipe,
    quantity: u64,
    start_tick: u64,
}

/// Очередь производственных заданий.
///
/// Завершённые задания выдаются в порядке добавления в очередь.
#[derive(Debug, Clone, Default)]
pub struct ProductionQueue {
    jobs: Vec<Job>,
}

impl ProductionQueue {
    /// Создаёт пустую очередь.
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    /// Добавляет задание на `quantity` запусков рецепта, стартующее в `start_tick`.
    ///
    /// Нулевое количество является операцией без эффекта.
    pub fn enqueue(&mut self, recipe: Recipe, quantity: u64, start_tick: u64) {
        if quantity == 0 {
            return;
        }
        self.jobs.push(Job {
            recipe,
            quantity,
            start_tick,
        });
    }

    /// Завершает все задания, готовые к тику `current_tick`.
    ///
    /// Задание готово, если `current_tick >= start_tick + recipe.time_ticks`
    /// (сумма насыщается на `u64::MAX`). Завершённые задания удаляются из очереди
    /// и возвращаются в порядке добавления; остальные, в том числе со стартом
    /// в будущем, остаются. Выходы умножаются на `quantity` с насыщением.
    pub fn tick(&mut self, current_tick: u64) -> Vec<CompletedBatch> {
        let mut completed = Vec::new();
        let mut remaining = Vec::new();

        for job in std::mem::take(&mut self.jobs) {
            let finish_tick = job.start_tick.saturating_add(job.recipe.time_ticks);
            if current_tick >= finish_tick {
                let outputs = job
                    .recipe
                    .outputs
                    .iter()
                    .map(|(&item, &qty)| (item, qty.saturating_mul(job.quantity)))
                    .collect();
                completed.push(CompletedBatch {
                    recipe_id: job.recipe.id,
                    quantity: job.quantity,
                    outputs,
                });
            } else {
                remaining.push(job);
            }
        }

        self.jobs = remaining;
        completed
    }

    /// Возвращает количество заданий в очереди.
    pub fn pending_jobs(&self) -> usize {
        self.jobs.len()
    }
}

/// Добавляет в `inventory` все выходы завершённого задания.
pub fn apply_batch(batch: &CompletedBatch, inventory: &mut Cargo) {
    for (&item, &qty) in &batch.outputs {
        inventory.add(item, qty);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(id: u32, outputs: &[(u64, u64)], time_ticks: u64) -> Recipe {
        Recipe {
            id,
            inputs: BTreeMap::new(),
            outputs: outputs.iter().map(|&(i, q)| (ItemId(i), q)).collect(),
            time_ticks,
        }
    }

    #[test]
    fn enqueue_zero_quantity_adds_nothing() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(recipe(1, &[(1, 5)], 10), 0, 0);
        assert_eq!(queue.pending_jobs(), 0);
    }

    #[test]
    fn job_completes_exactly_at_finish_tick() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(recipe(1, &[(1, 5)], 10), 1, 0);

        assert!(queue.tick(5).is_empty());
        assert_eq!(queue.pending_jobs(), 1);
        assert!(queue.tick(9).is_empty());

        let done = queue.tick(10);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].recipe_id, 1);
        assert_eq!(done[0].quantity, 1);
        assert_eq!(done[0].outputs.get(&ItemId(1)), Some(&5));
    }

    #[test]
    fn outputs_scale_with_quantity() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(recipe(1, &[(1, 5), (2, 2)], 1), 3, 0);

        let done = queue.tick(1);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].outputs[&ItemId(1)], 15);
        assert_eq!(done[0].outputs[&ItemId(2)], 6);
    }

    #[test]
    fn completed_jobs_keep_enqueue_order() {
        let mut queue = ProductionQueue::new();
        // Первый в очереди завершается позже всех, но порядок выдачи — по добавлению.
        queue.enqueue(recipe(10, &[(1, 1)], 8), 1, 0);
        queue.enqueue(recipe(20, &[(1, 1)], 2), 1, 0);
        queue.enqueue(recipe(30, &[(1, 1)], 5), 1, 0);

        let ids: Vec<u32> = queue.tick(10).iter().map(|b| b.recipe_id).collect();
        assert_eq!(ids, vec![10, 20, 30]);
    }

    #[test]
    fn future_start_tick_does_not_complete_early() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(recipe(1, &[(1, 1)], 0), 1, 100);

        assert!(queue.tick(50).is_empty());
        assert!(queue.tick(99).is_empty());
        assert_eq!(queue.pending_jobs(), 1);

        assert_eq!(queue.tick(100).len(), 1);
    }

    #[test]
    fn completed_job_is_removed_from_queue() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(recipe(1, &[(1, 1)], 3), 1, 0);
        queue.enqueue(recipe(2, &[(1, 1)], 10), 1, 0);
        assert_eq!(queue.pending_jobs(), 2);

        assert_eq!(queue.tick(3).len(), 1);
        assert_eq!(queue.pending_jobs(), 1);
        // Повторный тик не возвращает уже завершённое задание.
        assert!(queue.tick(3).is_empty());

        assert_eq!(queue.tick(10).len(), 1);
        assert_eq!(queue.pending_jobs(), 0);
    }

    #[test]
    fn apply_batch_adds_outputs_to_cargo() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(recipe(1, &[(1, 5), (2, 3)], 1), 2, 0);
        let done = queue.tick(1);

        let mut cargo = Cargo::new();
        cargo.add(ItemId(1), 4);
        apply_batch(&done[0], &mut cargo);

        assert_eq!(cargo.get(ItemId(1)), 14);
        assert_eq!(cargo.get(ItemId(2)), 6);
        assert_eq!(cargo.get(ItemId(3)), 0);
    }

    #[test]
    fn overflow_is_saturating_not_panicking() {
        let mut queue = ProductionQueue::new();
        queue.enqueue(recipe(1, &[(1, u64::MAX)], u64::MAX), 2, u64::MAX);

        let done = queue.tick(u64::MAX);
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].outputs[&ItemId(1)], u64::MAX);
    }

    #[test]
    fn identical_input_gives_identical_results() {
        let run = || {
            let mut queue = ProductionQueue::new();
            for id in 0..10u32 {
                let time = u64::from(id % 4) + 1;
                queue.enqueue(recipe(id, &[(1, 2), (2, 3)], time), u64::from(id) + 1, 0);
            }
            (1..=5).flat_map(|t| queue.tick(t)).collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn recipe_clone_equals_and_outputs_iterate_in_key_order() {
        let original = recipe(7, &[(3, 1), (1, 2)], 4);
        let cloned = original.clone();
        assert_eq!(original, cloned);
        let keys: Vec<u64> = original.outputs.keys().map(|k| k.0).collect();
        assert_eq!(keys, vec![1, 3]);
    }
}
