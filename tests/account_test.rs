use rust_matching_engine::account::AccountManager;
use rust_matching_engine::models::Side;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

#[test]
fn test_account_manager_load_and_get_balance() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();

    am.load_balance(user_id, "USDC", dec!(1000), dec!(0));
    
    let (avail, locked) = am.get_balance(&user_id, "USDC");
    assert_eq!(avail, dec!(1000));
    assert_eq!(locked, dec!(0));

    let (avail_bad, _) = am.get_balance(&user_id, "BAD");
    assert_eq!(avail_bad, dec!(0));
}

#[test]
fn test_try_lock_balance_buy() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    // 1000 USDC available
    am.load_balance(user_id, "USDC", dec!(1000), dec!(0));

    // Try to buy 10 items at price 50. Cost = 500 USDC.
    let res = am.try_lock_balance(&user_id, Side::Buy, dec!(50), 10);
    assert!(res.is_ok());

    let (avail, locked) = am.get_balance(&user_id, "USDC");
    assert_eq!(avail, dec!(500));
    assert_eq!(locked, dec!(500));
}

#[test]
fn test_try_lock_balance_buy_insufficient() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    am.load_balance(user_id, "USDC", dec!(100), dec!(0));

    // Cost = 500 USDC
    let res = am.try_lock_balance(&user_id, Side::Buy, dec!(50), 10);
    assert!(res.is_err());

    let (avail, locked) = am.get_balance(&user_id, "USDC");
    assert_eq!(avail, dec!(100)); // Unchanged
    assert_eq!(locked, dec!(0));
}

#[test]
fn test_try_lock_balance_sell() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    // 20 BAD available
    am.load_balance(user_id, "BAD", dec!(20), dec!(0));

    // Try to sell 10 items. Locks 10 BAD.
    let res = am.try_lock_balance(&user_id, Side::Sell, dec!(50), 10);
    assert!(res.is_ok());

    let (avail, locked) = am.get_balance(&user_id, "BAD");
    assert_eq!(avail, dec!(10));
    assert_eq!(locked, dec!(10));
}

#[test]
fn test_on_trade_match_buy() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    
    // Initial: 1000 USDC. Lock 500 for buy order.
    am.load_balance(user_id, "USDC", dec!(500), dec!(500));
    am.load_balance(user_id, "BAD", dec!(0), dec!(0));

    // Trade matches: Bought 10 @ 50.
    am.on_trade_match(&user_id, Side::Buy, dec!(50), 10);

    let (usdc_avail, usdc_locked) = am.get_balance(&user_id, "USDC");
    // Locked USDC consumed.
    assert_eq!(usdc_locked, dec!(0));
    // NOTE: In the current implementation, available USDC doesn't change on exact match (consumed from locked).
    // If trade price < order price, refund logic would be needed, but simplified version just consumes locked.
    assert_eq!(usdc_avail, dec!(500)); 

    let (bad_avail, bad_locked) = am.get_balance(&user_id, "BAD");
    // Received 10 BAD
    assert_eq!(bad_avail, dec!(10));
    assert_eq!(bad_locked, dec!(0));
}

#[test]
fn test_on_trade_match_sell() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    
    // Initial: 20 BAD. Lock 10 for sell order.
    am.load_balance(user_id, "BAD", dec!(10), dec!(10));
    am.load_balance(user_id, "USDC", dec!(0), dec!(0));

    // Trade matches: Sold 10 @ 50. Total value 500 USDC.
    am.on_trade_match(&user_id, Side::Sell, dec!(50), 10);

    let (bad_avail, bad_locked) = am.get_balance(&user_id, "BAD");
    // Locked BAD consumed
    assert_eq!(bad_locked, dec!(0));
    assert_eq!(bad_avail, dec!(10));

    let (usdc_avail, usdc_locked) = am.get_balance(&user_id, "USDC");
    // Received 500 USDC
    assert_eq!(usdc_avail, dec!(500));
    assert_eq!(usdc_locked, dec!(0));
}
