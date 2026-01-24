use rust_matching_engine::models::{Order, Side, OrderType};
use rust_matching_engine::orderbook::OrderBook;
use rust_decimal::Decimal;

// Helper to create Decimal from integer
fn deci(i: i64) -> Decimal {
    Decimal::from(i)
}

fn create_limit_order(id: u64, price: Decimal, quantity: u64, side: Side) -> Order {
    Order {
        id,
        price,
        quantity,
        side,
        user_id: None,
        order_type: OrderType::Limit,
    }
}

fn create_market_order(id: u64, quantity: u64, side: Side) -> Order {
    Order {
        id,
        price: Decimal::ZERO, // Market order has no price
        quantity,
        side,
        user_id: None,
        order_type: OrderType::Market,
    }
}

#[test]
fn test_market_buy_fills_multiple_levels() {
    let mut ob = OrderBook::new();
    // Sell Orders (Asks):
    // 10 @ 100
    // 10 @ 101
    ob.process_order(create_limit_order(1, deci(100), 10, Side::Sell));
    ob.process_order(create_limit_order(2, deci(101), 10, Side::Sell));

    // Market Buy 15
    // Should take 10 @ 100 and 5 @ 101
    let market_order = create_market_order(3, 15, Side::Buy);
    let trades = ob.process_order(market_order);

    assert_eq!(trades.len(), 2);
    
    // Trade 1: 10 @ 100
    assert_eq!(trades[0].price, deci(100));
    assert_eq!(trades[0].quantity, 10);
    assert_eq!(trades[0].maker_id, 1);

    // Trade 2: 5 @ 101
    assert_eq!(trades[1].price, deci(101));
    assert_eq!(trades[1].quantity, 5);
    assert_eq!(trades[1].maker_id, 2);

    // Remaining asks: 5 @ 101
    assert_eq!(ob.asks.get(&deci(101)).unwrap()[0].quantity, 5);
    // Order 1 at 100 should be gone
    assert!(ob.asks.get(&deci(100)).is_none());
}

#[test]
fn test_market_buy_no_liquidity() {
    let mut ob = OrderBook::new();
    // Empty order book
    
    let market_order = create_market_order(1, 10, Side::Buy);
    let trades = ob.process_order(market_order);

    // Should be no trades
    assert!(trades.is_empty());
    
    // Market order should NOT remain in the book
    assert!(ob.bids.is_empty());
    assert!(ob.asks.is_empty());
}
