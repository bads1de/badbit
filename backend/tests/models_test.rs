use rust_matching_engine::models::{Order, Side, OrderType};
use rust_decimal_macros::dec;
use serde_json::json;

#[test]
fn test_order_serialization() {
    let order = Order {
        id: 1,
        price: dec!(100.50),
        quantity: 10,
        side: Side::Buy,
        user_id: None,
        order_type: OrderType::Limit,
    };

    let json_str = serde_json::to_string(&order).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // Check that price is serialized as a string
    assert!(json_val["price"].is_string());
    assert_eq!(json_val["price"], "100.50");
    
    // Check other fields
    assert_eq!(json_val["id"], 1);
    assert_eq!(json_val["quantity"], 10);
    assert_eq!(json_val["side"], "Buy");
}

#[test]
fn test_order_deserialization() {
    let json_data = json!({
        "id": 2,
        "price": "99.99",
        "quantity": 5,
        "side": "Sell",
        "user_id": null
    });

    let order: Order = serde_json::from_value(json_data).unwrap();

    assert_eq!(order.id, 2);
    assert_eq!(order.price, dec!(99.99));
    assert_eq!(order.quantity, 5);
    assert_eq!(order.side, Side::Sell);
    assert!(order.user_id.is_none());
}
