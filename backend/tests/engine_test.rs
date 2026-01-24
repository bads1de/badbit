use rust_matching_engine::engine::{run_matching_engine, EngineMessage};
use rust_matching_engine::account::AccountManager;
use rust_matching_engine::db::DbMessage;
use rust_matching_engine::models::{Order, Side, OrderType};
use rust_decimal_macros::dec;
use uuid::Uuid;
use tokio::sync::{broadcast, mpsc, oneshot};

#[tokio::test]
async fn test_engine_place_order_no_match() {
    let (eng_tx, eng_rx) = mpsc::channel(10);
    let (db_tx, mut db_rx) = mpsc::channel(10);
    let (broadcast_tx, _) = broadcast::channel(100);
    let user_id = Uuid::new_v4();
    let mut am = AccountManager::new();
    am.load_balance(user_id, "BAD", dec!(100), dec!(0));
    
    tokio::spawn(async move {
        run_matching_engine(eng_rx, db_tx, am, broadcast_tx).await;
    });

    let (resp_tx, resp_rx) = oneshot::channel();
    eng_tx.send(EngineMessage::PlaceOrder { 
        order: Order { id: 1, price: dec!(100), quantity: 10, side: Side::Sell, user_id: Some(user_id), order_type: OrderType::Limit }, 
        respond_to: resp_tx 
    }).await.unwrap();

    let trades = resp_rx.await.unwrap();
    assert!(trades.is_empty());

    match db_rx.recv().await {
        Some(DbMessage::UpdateBalance { user_id: uid, asset, available, locked }) => {
            assert_eq!(uid, user_id);
            assert_eq!(asset, "BAD");
            assert_eq!(available, dec!(90));
            assert_eq!(locked, dec!(10));
        },
        _ => panic!("Expected UpdateBalance"),
    }
}

#[tokio::test]
async fn test_engine_match_trade() {
    let (eng_tx, eng_rx) = mpsc::channel(10);
    let (db_tx, mut db_rx) = mpsc::channel(10);
    let (broadcast_tx, _) = broadcast::channel(100);

    let maker_id = Uuid::new_v4();
    let taker_id = Uuid::new_v4();
    let mut am = AccountManager::new();
    
    am.load_balance(maker_id, "BAD", dec!(100), dec!(0));
    am.load_balance(taker_id, "USDC", dec!(10000), dec!(0));

    tokio::spawn(async move {
        run_matching_engine(eng_rx, db_tx, am, broadcast_tx).await;
    });

    // 1. Place Maker Order
    let (resp_tx1, resp_rx1) = oneshot::channel();
    eng_tx.send(EngineMessage::PlaceOrder { 
        order: Order { id: 1, price: dec!(100), quantity: 10, side: Side::Sell, user_id: Some(maker_id), order_type: OrderType::Limit }, 
        respond_to: resp_tx1 
    }).await.unwrap();
    let _ = resp_rx1.await.unwrap();
    
    // Verify Maker's DB update
    match db_rx.recv().await {
        Some(DbMessage::UpdateBalance { user_id, .. }) => {
            assert_eq!(user_id, maker_id, "First message should be for maker");
        },
        m => panic!("Expected Maker UpdateBalance, got {:?}", m),
    }

    // 2. Place Taker Order
    let (resp_tx2, resp_rx2) = oneshot::channel();
    eng_tx.send(EngineMessage::PlaceOrder { 
        order: Order { id: 2, price: dec!(100), quantity: 10, side: Side::Buy, user_id: Some(taker_id), order_type: OrderType::Limit }, 
        respond_to: resp_tx2 
    }).await.unwrap();
    
    let trades = resp_rx2.await.unwrap();
    assert_eq!(trades.len(), 1);

    // 3. Verify DB updates for Taker
    // Expect: Lock UpdateBalance -> SaveTrade -> Final UpdateBalance (USDC) -> Final UpdateBalance (BAD)
    
    // A. Lock Update (Taker)
    match db_rx.recv().await {
        Some(DbMessage::UpdateBalance { user_id, .. }) => {
             assert_eq!(user_id, taker_id, "Lock message should be for taker");
        },
        m => panic!("Expected Taker Lock UpdateBalance, got {:?}", m),
    }

    // B. Save Trade
    match db_rx.recv().await {
        Some(DbMessage::SaveTrade { user_id, .. }) => {
             assert_eq!(user_id, Some(taker_id));
        },
        m => panic!("Expected SaveTrade, got {:?}", m),
    }
}