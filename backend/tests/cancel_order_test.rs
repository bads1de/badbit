use rust_matching_engine::engine::{run_matching_engine, EngineMessage};
use rust_matching_engine::account::AccountManager;
use rust_matching_engine::db::DbMessage;
use rust_matching_engine::models::{Order, Side, OrderType};
use rust_decimal_macros::dec;
use uuid::Uuid;
use tokio::sync::{broadcast, mpsc, oneshot};

#[tokio::test]
async fn test_cancel_order_releases_funds() {
    let (eng_tx, eng_rx) = mpsc::channel(10);
    let (db_tx, mut db_rx) = mpsc::channel(10);
    let (broadcast_tx, _) = broadcast::channel(100);

    let user_id = Uuid::new_v4();
    let mut am = AccountManager::new();
    
    // 初期残高: 1000 USDC
    am.load_balance(user_id, "USDC", dec!(1000), dec!(0));

    tokio::spawn(async move {
        run_matching_engine(eng_rx, db_tx, am, broadcast_tx).await;
    });

    // 1. 注文 (100 * 5 = 500 USDC ロック)
    let (resp_tx, resp_rx) = oneshot::channel();
    let order_id = 1;
    eng_tx.send(EngineMessage::PlaceOrder {
        order: Order { id: order_id, price: dec!(100), quantity: 5, side: Side::Buy, user_id: Some(user_id), order_type: OrderType::Limit },
        respond_to: resp_tx
    }).await.unwrap();
    let _ = resp_rx.await.unwrap();

    // ロック確認 (DBMessage)
    match db_rx.recv().await {
        Some(DbMessage::UpdateBalance { user_id: uid, asset, available, locked }) => {
            assert_eq!(uid, user_id);
            assert_eq!(asset, "USDC");
            assert_eq!(available, dec!(500));
            assert_eq!(locked, dec!(500));
        },
        _ => panic!("Expected Lock UpdateBalance"),
    }

    // 2. キャンセル実行
    let (cancel_resp_tx, cancel_resp_rx) = oneshot::channel();
    eng_tx.send(EngineMessage::CancelOrder {
        order_id,
        user_id,
        respond_to: cancel_resp_tx
    }).await.unwrap();

    let canceled_order = cancel_resp_rx.await.unwrap();
    assert!(canceled_order.is_some());
    let o = canceled_order.unwrap();
    assert_eq!(o.id, order_id);

    // 3. 残高解除の確認 (DBMessageを受け取るはず)
    match db_rx.recv().await {
        Some(DbMessage::UpdateBalance { user_id: uid, asset, available, locked }) => {
            assert_eq!(uid, user_id);
            assert_eq!(asset, "USDC");
            assert_eq!(available, dec!(1000)); // 元に戻る
            assert_eq!(locked, dec!(0));
        },
        _ => panic!("Expected Unlock UpdateBalance"),
    }
}
