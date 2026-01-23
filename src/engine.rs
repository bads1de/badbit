use tokio::sync::{mpsc, oneshot};
use crate::models::{Order, Trade, Side};
use crate::orderbook::OrderBook;
use crate::account::AccountManager;
use crate::db::DbMessage;

// =============================================================================
// Actorパターンのメッセージ定義
// =============================================================================
// 
// Actorパターンでは、データを持つ「アクター」にメッセージを送って操作を依頼します。
// 直接データにアクセスするのではなく、「〇〇してください」というメッセージを送り、
// アクターが自分のタイミングで処理して結果を返します。
// 
// これによりロックなしで安全な並行処理が実現できます。

/// エンジン（アクター）に送るメッセージの種類を定義
/// 
/// 各バリアントは「依頼の種類」と「結果の返信先」を持ちます。
/// respond_toフィールドがoneshot::Senderなのは:
/// - 1つのリクエストに対して1つの応答だけが返るため
/// - 送信後にチャネルは閉じられる（再利用不可）
pub enum EngineMessage {
    /// 新規注文を処理してください
    PlaceOrder {
        order: Order,                          // 処理してほしい注文
        respond_to: oneshot::Sender<Vec<Trade>>, // 約定リストを返信する先
    },
    /// 現在のオーダーブックを見せてください
    GetOrderBook {
        respond_to: oneshot::Sender<OrderBook>,
    },
    /// 取引履歴を見せてください
    GetTrades {
        respond_to: oneshot::Sender<Vec<Trade>>,
    },
}

/// マッチングエンジンを実行する（Actor Loop）
pub async fn run_matching_engine(
    mut rx: mpsc::Receiver<EngineMessage>,
    db_tx: mpsc::Sender<DbMessage>,
    mut account_manager: AccountManager,
) {
    let mut orderbook = OrderBook::new();
    let mut trades_history: Vec<Trade> = Vec::new();
    // account_managerはmoveされる（所有権がこのタスクに移る）

    while let Some(msg) = rx.recv().await {
        match msg {
            EngineMessage::PlaceOrder { order, respond_to } => {
                // 1. 残高チェック & ロック
                if let Some(uid) = order.user_id {
                    if let Err(e) = account_manager.try_lock_balance(&uid, order.side, order.price, order.quantity) {
                        eprintln!("Order Rejected: {}", e);
                        // エラー時は空のトレードリストを返して終了
                        let _ = respond_to.send(vec![]);
                        continue;
                    }
                    // ロック成功 → DBに通知
                    // 注意: ここのロック状態も永続化すべきだが、厳密には「注文ID」と紐づける必要がある。
                    // 今回は簡易的に残高だけ更新通知を送る。
                    let (avail, locked) = account_manager.get_balance(&uid, if order.side == Side::Buy { "USDC" } else { "BAD" });
                    let _ = db_tx.send(DbMessage::UpdateBalance { 
                        user_id: uid, 
                        asset: (if order.side == Side::Buy { "USDC" } else { "BAD" }).to_string(), 
                        available: avail, 
                        locked 
                    }).await;
                }

                // 2. マッチング実行
                let new_trades = orderbook.process_order(order.clone());
                
                // 3. 約定処理 (残高移動)
                for _trade in &new_trades {
                    // Maker（板にいた人）の処理
                    // シミュレータの注文(user_id=None)は無視する
                    // しかし、注文IDから元のUserを探す仕組みがまだないため、
                    // ここでは「今回のTaker」がユーザーの場合のみ処理する簡易実装とする
                    // ★ 本来は OrderBook内の Order に user_id が入っているので、それを使うべき
                    // process_order の返り値 Trade には user_id がない。これが必要。
                }
                
                // ★ Trade構造体に user_id を持たせていないため、ここで詰まる。
                // 修正: Trade構造体に user_id はあるが、maker/takerのどちらか不明確。
                // 正しい実装: process_order が返す Trade には maker_order と taker_order の情報が必要。
                // ここでロジックを修正する必要がある。
                
                // 今回は Taker (注文を出した人) の残高更新だけを行う（Makerはシミュレータと仮定）
                    if let Some(taker_uid) = order.user_id {
                    for trade in &new_trades {
                        // Takerの残高更新
                        account_manager.on_trade_match(&taker_uid, order.side, trade.price, trade.quantity);
                        
                        // DBに保存
                        let _ = db_tx.send(DbMessage::SaveTrade {
                            maker_order_id: trade.maker_id,
                            taker_order_id: trade.taker_id,
                            price: trade.price,
                            quantity: trade.quantity,
                            timestamp: trade.timestamp,
                            user_id: Some(taker_uid),
                        }).await;
                    }
                    
                    // 残高変更をDBに通知 (USDCとBAD両方)
                    let (usdc_av, usdc_lk) = account_manager.get_balance(&taker_uid, "USDC");
                    let _ = db_tx.send(DbMessage::UpdateBalance { user_id: taker_uid, asset: "USDC".to_string(), available: usdc_av, locked: usdc_lk }).await;
                    
                    let (bad_av, bad_lk) = account_manager.get_balance(&taker_uid, "BAD");
                    let _ = db_tx.send(DbMessage::UpdateBalance { user_id: taker_uid, asset: "BAD".to_string(), available: bad_av, locked: bad_lk }).await;
                }

                trades_history.extend(new_trades.clone());
                let _ = respond_to.send(new_trades);
            },

            EngineMessage::GetOrderBook { respond_to } => {
                let _ = respond_to.send(orderbook.clone());
            },
            EngineMessage::GetTrades { respond_to } => {
                let _ = respond_to.send(trades_history.clone());
            }
        }
        
        if trades_history.len() > 5000 {
            let tail = trades_history.len() - 2000;
            trades_history.drain(0..tail);
        }
    }
}
