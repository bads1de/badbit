use rust_matching_engine::models::{Order, Side, OrderType};
use rust_matching_engine::orderbook::OrderBook;
use rust_decimal::Decimal;

// Helper to create Decimal from integer
fn deci(i: i64) -> Decimal {
    Decimal::from(i)
}

fn create_order(id: u64, price: Decimal, quantity: u64, side: Side) -> Order {
    Order {
        id,
        price,
        quantity,
        side,
        user_id: None,
        order_type: OrderType::Limit,
    }
}

#[test]
fn test_place_limit_buy_order_no_match() {
    let mut ob = OrderBook::new();
    let order = create_order(1, deci(100), 10, Side::Buy);
    
    let trades = ob.process_order(order);

    assert!(trades.is_empty());
    // Since fields might not be pub, we rely on public methods or pub fields.
    // OrderBook fields `bids` and `asks` are public in the file I viewed.
    assert_eq!(ob.bids.len(), 1);
    assert_eq!(ob.asks.len(), 0);
    assert_eq!(ob.bids.get(&deci(100)).unwrap().len(), 1);
    assert_eq!(ob.bids.get(&deci(100)).unwrap()[0].quantity, 10);
}

#[test]
fn test_place_limit_sell_order_no_match() {
    let mut ob = OrderBook::new();
    let order = create_order(1, deci(100), 10, Side::Sell);
    
    let trades = ob.process_order(order);

    assert!(trades.is_empty());
    assert_eq!(ob.asks.len(), 1);
    assert_eq!(ob.bids.len(), 0);
    assert_eq!(ob.asks.get(&deci(100)).unwrap().len(), 1);
    assert_eq!(ob.asks.get(&deci(100)).unwrap()[0].quantity, 10);
}

#[test]
fn test_full_match_buy_taker() {
    let mut ob = OrderBook::new();
    // Maker sell order: price 100, qty 10
    ob.process_order(create_order(1, deci(100), 10, Side::Sell));

    // Taker buy order: price 100, qty 10
    let taker_order = create_order(2, deci(100), 10, Side::Buy);
    let trades = ob.process_order(taker_order);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].maker_id, 1);
    assert_eq!(trades[0].taker_id, 2);
    assert_eq!(trades[0].price, deci(100));
    assert_eq!(trades[0].quantity, 10);

    // Both order books should be empty
    assert!(ob.bids.is_empty());
    assert!(ob.asks.is_empty());
}

#[test]
fn test_full_match_sell_taker() {
    let mut ob = OrderBook::new();
    // Maker buy order: price 100, qty 10
    ob.process_order(create_order(1, deci(100), 10, Side::Buy));

    // Taker sell order: price 100, qty 10
    let taker_order = create_order(2, deci(100), 10, Side::Sell);
    let trades = ob.process_order(taker_order);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].maker_id, 1);
    assert_eq!(trades[0].taker_id, 2);
    assert_eq!(trades[0].price, deci(100));
    assert_eq!(trades[0].quantity, 10);

    assert!(ob.bids.is_empty());
    assert!(ob.asks.is_empty());
}

#[test]
fn test_partial_match_maker_remains() {
    let mut ob = OrderBook::new();
    // Maker sell order: price 100, qty 20
    ob.process_order(create_order(1, deci(100), 20, Side::Sell));

    // Taker buy order: price 100, qty 10
    let trades = ob.process_order(create_order(2, deci(100), 10, Side::Buy));

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].quantity, 10);

    // Asks should still have 10 remaining at price 100
    assert_eq!(ob.asks.get(&deci(100)).unwrap()[0].quantity, 10);
    assert!(ob.bids.is_empty());
}

#[test]
fn test_partial_match_taker_remains() {
    let mut ob = OrderBook::new();
    // Maker sell order: price 100, qty 10
    ob.process_order(create_order(1, deci(100), 10, Side::Sell));

    // Taker buy order: price 100, qty 20
    let trades = ob.process_order(create_order(2, deci(100), 20, Side::Buy));

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].quantity, 10);

    // Taker remainder should be in bids
    assert_eq!(ob.bids.get(&deci(100)).unwrap()[0].quantity, 10);
    assert!(ob.asks.is_empty());
}

#[test]
fn test_match_better_price() {
    let mut ob = OrderBook::new();
    // Maker sell order: price 90, qty 10 (willing to sell cheap)
    ob.process_order(create_order(1, deci(90), 10, Side::Sell));

    // Taker buy order: price 100, qty 10 (willing to buy expensive)
    // Should match at the maker's price (90)
    let trades = ob.process_order(create_order(2, deci(100), 10, Side::Buy));

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].price, deci(90)); // Match at maker price
    assert!(ob.asks.is_empty());
    assert!(ob.bids.is_empty());
}

#[test]
fn test_price_time_priority() {
    let mut ob = OrderBook::new();
    // Multiple sell orders at same price
    ob.process_order(create_order(1, deci(100), 10, Side::Sell)); // Order 1 (First)
    ob.process_order(create_order(2, deci(100), 10, Side::Sell)); // Order 2 (Second)

    // Taker buy matches order 1 first
    let trades = ob.process_order(create_order(3, deci(100), 15, Side::Buy));

    assert_eq!(trades.len(), 2);
    
    // First trade with Order 1
    assert_eq!(trades[0].maker_id, 1);
    assert_eq!(trades[0].quantity, 10);

    // Second trade with Order 2
    assert_eq!(trades[1].maker_id, 2);
    assert_eq!(trades[1].quantity, 5); // Remainder

    // Order 2 has 5 remaining
    assert_eq!(ob.asks.get(&deci(100)).unwrap()[0].quantity, 5);
    assert_eq!(ob.asks.get(&deci(100)).unwrap()[0].id, 2);
}
