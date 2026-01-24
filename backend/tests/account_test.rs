use rust_matching_engine::account::AccountManager;
use rust_matching_engine::models::Side;
use rust_decimal_macros::dec;
use uuid::Uuid;

#[test]
fn test_initial_balance() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();

    // 初期残高セット
    am.load_balance(user_id, "USDC", dec!(1000), dec!(0));
    am.load_balance(user_id, "BAD", dec!(50), dec!(0));

    let (usdc_avail, usdc_locked) = am.get_balance(&user_id, "USDC");
    assert_eq!(usdc_avail, dec!(1000));
    assert_eq!(usdc_locked, dec!(0));

    let (bad_avail, bad_locked) = am.get_balance(&user_id, "BAD");
    assert_eq!(bad_avail, dec!(50));
    assert_eq!(bad_locked, dec!(0));
}

#[test]
fn test_lock_balance_buy() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    am.load_balance(user_id, "USDC", dec!(1000), dec!(0));

    // 買い注文: 価格 100 * 数量 5 = 500 USDC 必要
    let res = am.try_lock_balance(&user_id, Side::Buy, dec!(100), 5);
    
    assert!(res.is_ok());

    let (avail, locked) = am.get_balance(&user_id, "USDC");
    assert_eq!(avail, dec!(500)); // 1000 - 500
    assert_eq!(locked, dec!(500)); // 0 + 500
}

#[test]
fn test_lock_balance_sell() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    am.load_balance(user_id, "BAD", dec!(20), dec!(0));

    // 売り注文: 数量 10 BAD 必要
    let res = am.try_lock_balance(&user_id, Side::Sell, dec!(100), 10);
    
    assert!(res.is_ok());

    let (avail, locked) = am.get_balance(&user_id, "BAD");
    assert_eq!(avail, dec!(10)); // 20 - 10
    assert_eq!(locked, dec!(10)); // 0 + 10
}

#[test]
fn test_lock_insufficient_funds() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    am.load_balance(user_id, "USDC", dec!(100), dec!(0));

    // 残高 100 しかないのに 500 必要
    let res = am.try_lock_balance(&user_id, Side::Buy, dec!(100), 5);
    
    assert!(res.is_err());
    
    // 残高は変わっていないはず
    let (avail, locked) = am.get_balance(&user_id, "USDC");
    assert_eq!(avail, dec!(100));
    assert_eq!(locked, dec!(0));
}

#[test]
fn test_trade_match_buy() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    
    // 初期: 1000 USDC, 0 BAD
    am.load_balance(user_id, "USDC", dec!(1000), dec!(0));
    am.load_balance(user_id, "BAD", dec!(0), dec!(0));

    // 1. 注文でロック (100 * 5 = 500 USDC)
    am.try_lock_balance(&user_id, Side::Buy, dec!(100), 5).unwrap();

    // 2. 約定 (同じ価格で全量約定と仮定)
    am.on_trade_match(&user_id, Side::Buy, dec!(100), 5);

    // USDC: ロックされていた500が消費され、残りは500
    let (usdc_avail, usdc_locked) = am.get_balance(&user_id, "USDC");
    assert_eq!(usdc_avail, dec!(500));
    assert_eq!(usdc_locked, dec!(0)); // ロック解除

    // BAD: 5 BAD 入手
    let (bad_avail, bad_locked) = am.get_balance(&user_id, "BAD");
    assert_eq!(bad_avail, dec!(5));
    assert_eq!(bad_locked, dec!(0));
}

#[test]
fn test_trade_match_sell() {
    let mut am = AccountManager::new();
    let user_id = Uuid::new_v4();
    
    // 初期: 0 USDC, 10 BAD
    am.load_balance(user_id, "USDC", dec!(0), dec!(0));
    am.load_balance(user_id, "BAD", dec!(10), dec!(0));

    // 1. 注文でロック (10 BAD)
    am.try_lock_balance(&user_id, Side::Sell, dec!(100), 10).unwrap();

    // 2. 約定 (価格 100 で 10 枚売れた)
    am.on_trade_match(&user_id, Side::Sell, dec!(100), 10);

    // USDC: 100 * 10 = 1000 USDC 入手
    let (usdc_avail, _) = am.get_balance(&user_id, "USDC");
    assert_eq!(usdc_avail, dec!(1000));

    // BAD: ロックされていた10が消費され、0に
    let (bad_avail, bad_locked) = am.get_balance(&user_id, "BAD");
    assert_eq!(bad_avail, dec!(0));
    assert_eq!(bad_locked, dec!(0));
}
