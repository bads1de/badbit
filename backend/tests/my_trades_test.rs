use rust_matching_engine::engine::{run_matching_engine, EngineMessage};
use rust_matching_engine::account::AccountManager;
use rust_matching_engine::db;
use rust_matching_engine::models::{Order, Side, OrderType};
use rust_decimal_macros::dec;
use tokio::sync::{broadcast, mpsc, oneshot};

#[tokio::test]
async fn test_my_trades_retrieval() {
    // 1. テスト用DBを作成（メモリ内）
    let (db_pool, user_id) = db::init_database(":memory:").await.unwrap();
    let (db_tx, db_rx) = mpsc::channel(10);
    let db_pool_clone = db_pool.clone();

    // 2. DB Writerを起動
    tokio::spawn(async move {
        db::run_db_writer(db_rx, db_pool_clone).await;
    });

    // 3. Engineを起動
    let (eng_tx, eng_rx) = mpsc::channel(10);
    let (broadcast_tx, _) = broadcast::channel(100);
    let mut am = AccountManager::new();
    am.load_balance(user_id, "USDC", dec!(10000), dec!(0));
    am.load_balance(user_id, "BAD", dec!(10000), dec!(0));
    
    // EngineがDB Writerを使うように修正
    let eng_db_tx = db_tx.clone();
    tokio::spawn(async move {
        run_matching_engine(eng_rx, eng_db_tx, am, broadcast_tx).await;
    });

    // 4. 注文を出して約定させる
    // 売り注文 (Maker)
    let (resp_tx1, resp_rx1) = oneshot::channel();
    eng_tx.send(EngineMessage::PlaceOrder {
        order: Order { id: 1, price: dec!(100), quantity: 10, side: Side::Sell, user_id: Some(user_id), order_type: OrderType::Limit },
        respond_to: resp_tx1
    }).await.unwrap();
    let _ = resp_rx1.await.unwrap();

    // 買い注文 (Taker) - 自分の売り注文にぶつける（自己約定の形になるがDBには記録される）
    let (resp_tx2, resp_rx2) = oneshot::channel();
    eng_tx.send(EngineMessage::PlaceOrder {
        order: Order { id: 2, price: dec!(100), quantity: 5, side: Side::Buy, user_id: Some(user_id), order_type: OrderType::Limit },
        respond_to: resp_tx2
    }).await.unwrap();
    let _ = resp_rx2.await.unwrap();

    // DBへの書き込みを少し待つ
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 5. 自分の履歴を取得できるか確認
    let trades = db::get_user_trades(&db_pool, user_id).await.unwrap();
    
    assert_eq!(trades.len(), 1);
    let trade = &trades[0];
    assert_eq!(trade.maker_id, 1);
    assert_eq!(trade.taker_id, 2);
    assert_eq!(trade.price, dec!(100));
    assert_eq!(trade.quantity, 5);
}
