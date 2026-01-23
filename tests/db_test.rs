use rust_matching_engine::db::{init_database, get_balances, update_balance, save_trade};
use rust_decimal_macros::dec;
use uuid::Uuid;
use std::fs;

// ヘルパー: ランダムなDBパスを生成
fn temp_db_path() -> String {
    let id = Uuid::new_v4();
    format!("test_db_{}.sqlite", id)
}

#[tokio::test]
async fn test_db_init_and_default_user() {
    let db_path = temp_db_path();
    
    // 1. Init Database
    let (pool, default_user_id) = init_database(&db_path).await.expect("Failed to init db");

    // 2. Check default user balance
    let balances = get_balances(&pool, default_user_id).await.expect("Failed to get balances");
    
    // Default user should have USDC and BAD entries
    let usdc = balances.iter().find(|b| b.asset == "USDC").expect("USDC missing");
    assert_eq!(usdc.available, dec!(10000));
    assert_eq!(usdc.locked, dec!(0));

    let bad = balances.iter().find(|b| b.asset == "BAD").expect("BAD missing");
    assert_eq!(bad.available, dec!(0));
    assert_eq!(bad.locked, dec!(0));

    // Cleanup
    pool.close().await;
    let _ = fs::remove_file(db_path);
}

#[tokio::test]
async fn test_db_update_balance() {
    let db_path = temp_db_path();
    let (pool, user_id) = init_database(&db_path).await.expect("Failed to init db");

    // Update USDC balance
    update_balance(&pool, user_id, "USDC", dec!(5000), dec!(1000))
        .await
        .expect("Failed to update balance");

    // Verify
    let balances = get_balances(&pool, user_id).await.expect("Failed to get balances");
    let usdc = balances.iter().find(|b| b.asset == "USDC").unwrap();
    
    assert_eq!(usdc.available, dec!(5000));
    assert_eq!(usdc.locked, dec!(1000));

    // Cleanup
    pool.close().await;
    let _ = fs::remove_file(db_path);
}

#[tokio::test]
async fn test_db_save_trade() {
    let db_path = temp_db_path();
    let (pool, user_id) = init_database(&db_path).await.expect("Failed to init db");

    let maker_id = 100;
    let taker_id = 101;
    let price = dec!(150.5);
    let quantity = 10;
    let timestamp = 1234567890;

    save_trade(
        &pool,
        maker_id,
        taker_id,
        price,
        quantity,
        timestamp,
        Some(user_id),
    )
    .await
    .expect("Failed to save trade");

    // Verify directly with SQL query
    let row: (i64, i64, String, i64, i64, String) = sqlx::query_as(
        "SELECT maker_order_id, taker_order_id, price, quantity, timestamp, user_id FROM trades LIMIT 1"
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch trade");

    assert_eq!(row.0, maker_id as i64);
    assert_eq!(row.1, taker_id as i64);
    assert_eq!(row.2, "150.5"); // Stored as string
    assert_eq!(row.3, 10);
    assert_eq!(row.4, timestamp as i64);
    assert_eq!(row.5, user_id.to_string());

    // Cleanup
    pool.close().await;
    let _ = fs::remove_file(db_path);
}
