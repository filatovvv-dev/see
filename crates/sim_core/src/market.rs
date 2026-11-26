//! Рынок: книга ордеров, детерминированный матчинг и история цен.
//!
//! Все коллекции здесь детерминированы: используются только `Vec` и `BTreeMap`,
//! сортировка выполняется через `sort_by` с явным компаратором
//! (см. AGENTS.md, раздел «Главное правило детерминизма»).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::entities::{ItemId, MarketOrder};

/// Ошибка операции с рынком.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketError {
    /// Количество в ордере равно нулю.
    ZeroQuantity,
}

impl fmt::Display for MarketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MarketError::ZeroQuantity => write!(f, "order quantity must be greater than zero"),
        }
    }
}

impl std::error::Error for MarketError {}

/// Сделка, совершённая при матчинге ордеров.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    /// Предмет, которым торговали.
    pub item: ItemId,
    /// Цена сделки за единицу (цена ордера на продажу).
    pub price: u64,
    /// Количество единиц в сделке.
    pub quantity: u64,
    /// Идентификатор ордера покупателя.
    pub buyer_order_id: u64,
    /// Идентификатор ордера продавца.
    pub seller_order_id: u64,
}

/// Рынок: книга открытых ордеров и история цен по предметам.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Market {
    orders: Vec<MarketOrder>,
    price_history: BTreeMap<ItemId, Vec<u64>>,
}

impl Market {
    /// Создаёт пустой рынок.
    pub fn new() -> Self {
        Self {
            orders: Vec::new(),
            price_history: BTreeMap::new(),
        }
    }

    /// Размещает ордер в книге.
    ///
    /// Возвращает [`MarketError::ZeroQuantity`], если `quantity == 0`.
    /// Нулевая цена допустима (бесплатная раздача).
    pub fn place_order(&mut self, order: MarketOrder) -> Result<(), MarketError> {
        if order.quantity == 0 {
            return Err(MarketError::ZeroQuantity);
        }
        self.orders.push(order);
        Ok(())
    }

    /// Сопоставляет ордера и возвращает совершённые сделки в порядке исполнения.
    ///
    /// Предметы обрабатываются по возрастанию [`ItemId`]. Внутри предмета
    /// покупки упорядочены по убыванию цены, продажи — по возрастанию цены;
    /// при равенстве цен приоритет у меньшего `order_id`. Сделка происходит,
    /// если цена покупки не ниже цены продажи, по цене ордера на продажу.
    /// Частично исполненный ордер остаётся в книге с уменьшенным количеством,
    /// полностью исполненный удаляется.
    pub fn match_orders(&mut self) -> Vec<Trade> {
        let items: BTreeSet<ItemId> = self.orders.iter().map(|o| o.item).collect();
        let mut trades = Vec::new();

        for item in items {
            let mut buys: Vec<usize> = Vec::new();
            let mut sells: Vec<usize> = Vec::new();
            for (idx, order) in self.orders.iter().enumerate() {
                if order.item != item {
                    continue;
                }
                if order.is_buy {
                    buys.push(idx);
                } else {
                    sells.push(idx);
                }
            }

            buys.sort_by(|&a, &b| {
                let (oa, ob) = (&self.orders[a], &self.orders[b]);
                ob.price
                    .cmp(&oa.price)
                    .then_with(|| oa.order_id.cmp(&ob.order_id))
            });
            sells.sort_by(|&a, &b| {
                let (oa, ob) = (&self.orders[a], &self.orders[b]);
                oa.price
                    .cmp(&ob.price)
                    .then_with(|| oa.order_id.cmp(&ob.order_id))
            });

            let (mut bi, mut si) = (0, 0);
            while bi < buys.len() && si < sells.len() {
                let buy = self.orders[buys[bi]];
                let sell = self.orders[sells[si]];
                if buy.price < sell.price {
                    break;
                }

                let quantity = buy.quantity.min(sell.quantity);
                trades.push(Trade {
                    item,
                    price: sell.price,
                    quantity,
                    buyer_order_id: buy.order_id,
                    seller_order_id: sell.order_id,
                });

                self.orders[buys[bi]].quantity -= quantity;
                self.orders[sells[si]].quantity -= quantity;
                if self.orders[buys[bi]].quantity == 0 {
                    bi += 1;
                }
                if self.orders[sells[si]].quantity == 0 {
                    si += 1;
                }
            }
        }

        self.orders.retain(|o| o.quantity > 0);
        trades
    }

    /// Записывает цену `price` в историю предмета `item`.
    pub fn record_price(&mut self, item: ItemId, price: u64) {
        self.price_history.entry(item).or_default().push(price);
    }

    /// Возвращает последнюю записанную цену предмета, если она есть.
    pub fn last_price(&self, item: ItemId) -> Option<u64> {
        self.price_history
            .get(&item)
            .and_then(|prices| prices.last().copied())
    }

    /// Возвращает максимальную цену среди ордеров на покупку предмета.
    pub fn best_bid(&self, item: ItemId) -> Option<u64> {
        self.orders
            .iter()
            .filter(|o| o.is_buy && o.item == item)
            .map(|o| o.price)
            .max()
    }

    /// Возвращает минимальную цену среди ордеров на продажу предмета.
    pub fn best_ask(&self, item: ItemId) -> Option<u64> {
        self.orders
            .iter()
            .filter(|o| !o.is_buy && o.item == item)
            .map(|o| o.price)
            .min()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM: ItemId = ItemId(1);

    fn order(order_id: u64, item: ItemId, price: u64, quantity: u64, is_buy: bool) -> MarketOrder {
        MarketOrder {
            order_id,
            item,
            price,
            quantity,
            is_buy,
        }
    }

    #[test]
    fn full_match_removes_both_orders() {
        let mut market = Market::new();
        market.place_order(order(1, ITEM, 50, 100, true)).unwrap();
        market.place_order(order(2, ITEM, 40, 100, false)).unwrap();

        let trades = market.match_orders();

        assert_eq!(
            trades,
            vec![Trade {
                item: ITEM,
                price: 40,
                quantity: 100,
                buyer_order_id: 1,
                seller_order_id: 2,
            }]
        );
        assert_eq!(market.best_bid(ITEM), None);
        assert_eq!(market.best_ask(ITEM), None);
    }

    #[test]
    fn partial_match_keeps_remainder_of_buy() {
        let mut market = Market::new();
        market.place_order(order(1, ITEM, 50, 100, true)).unwrap();
        market.place_order(order(2, ITEM, 40, 60, false)).unwrap();

        let trades = market.match_orders();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 40);
        assert_eq!(trades[0].quantity, 60);
        assert_eq!(market.best_bid(ITEM), Some(50));
        assert_eq!(market.best_ask(ITEM), None);
        assert_eq!(market.orders.len(), 1);
        assert_eq!(market.orders[0].order_id, 1);
        assert_eq!(market.orders[0].quantity, 40);
    }

    #[test]
    fn equal_price_priority_by_order_id() {
        let mut market = Market::new();
        // Намеренно кладём ордер с большим id первым.
        market.place_order(order(2, ITEM, 50, 10, true)).unwrap();
        market.place_order(order(1, ITEM, 50, 10, true)).unwrap();
        market.place_order(order(3, ITEM, 40, 10, false)).unwrap();

        let trades = market.match_orders();

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].buyer_order_id, 1);
        assert_eq!(market.orders.len(), 1);
        assert_eq!(market.orders[0].order_id, 2);
    }

    #[test]
    fn no_match_when_prices_do_not_cross() {
        let mut market = Market::new();
        market.place_order(order(1, ITEM, 50, 100, true)).unwrap();
        market.place_order(order(2, ITEM, 100, 100, false)).unwrap();

        let trades = market.match_orders();

        assert!(trades.is_empty());
        assert_eq!(market.orders.len(), 2);
        assert_eq!(market.best_bid(ITEM), Some(50));
        assert_eq!(market.best_ask(ITEM), Some(100));
    }

    #[test]
    fn zero_quantity_is_rejected() {
        let mut market = Market::new();
        let result = market.place_order(order(1, ITEM, 50, 0, true));
        assert_eq!(result, Err(MarketError::ZeroQuantity));
        assert!(market.orders.is_empty());
    }

    #[test]
    fn zero_price_is_accepted() {
        let mut market = Market::new();
        assert_eq!(market.place_order(order(1, ITEM, 0, 5, false)), Ok(()));
        assert_eq!(market.best_ask(ITEM), Some(0));
    }

    #[test]
    fn record_price_and_last_price() {
        let mut market = Market::new();
        assert_eq!(market.last_price(ITEM), None);
        market.record_price(ITEM, 10);
        market.record_price(ITEM, 12);
        assert_eq!(market.last_price(ITEM), Some(12));
        assert_eq!(market.last_price(ItemId(2)), None);
    }

    #[test]
    fn empty_book_has_no_bid_or_ask() {
        let market = Market::new();
        assert_eq!(market.best_bid(ITEM), None);
        assert_eq!(market.best_ask(ITEM), None);
    }

    #[test]
    fn one_buy_sweeps_several_sells_in_price_order() {
        let mut market = Market::new();
        market.place_order(order(1, ITEM, 60, 50, true)).unwrap();
        market.place_order(order(2, ITEM, 55, 20, false)).unwrap();
        market.place_order(order(3, ITEM, 45, 20, false)).unwrap();
        market.place_order(order(4, ITEM, 45, 20, false)).unwrap();

        let trades = market.match_orders();

        let summary: Vec<(u64, u64, u64)> = trades
            .iter()
            .map(|t| (t.seller_order_id, t.price, t.quantity))
            .collect();
        assert_eq!(summary, vec![(3, 45, 20), (4, 45, 20), (2, 55, 10)]);
        assert_eq!(market.orders.len(), 1);
        assert_eq!(market.orders[0].order_id, 2);
        assert_eq!(market.orders[0].quantity, 10);
    }

    #[test]
    fn different_items_do_not_match() {
        let mut market = Market::new();
        market
            .place_order(order(1, ItemId(1), 50, 10, true))
            .unwrap();
        market
            .place_order(order(2, ItemId(2), 10, 10, false))
            .unwrap();

        assert!(market.match_orders().is_empty());
        assert_eq!(market.orders.len(), 2);
    }

    #[test]
    fn matching_is_deterministic() {
        let build = || {
            let mut market = Market::new();
            for id in 0..20u64 {
                let item = ItemId(id % 3);
                let is_buy = id % 2 == 0;
                let price = 30 + (id * 7) % 25;
                market
                    .place_order(order(id, item, price, 1 + id % 4, is_buy))
                    .unwrap();
            }
            market
        };

        let trades_a = build().match_orders();
        let trades_b = build().match_orders();
        assert_eq!(trades_a, trades_b);
    }

    #[test]
    fn error_display_message() {
        assert_eq!(
            MarketError::ZeroQuantity.to_string(),
            "order quantity must be greater than zero"
        );
    }
}
