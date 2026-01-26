// =============================================================================
// badbit - 取引マッチングエンジン
// =============================================================================
//
// このプログラムは、仮想通貨取引所のオーダーブック（板）とマッチングエンジンを
// シミュレートするWebサーバーです。
//
// 【アーキテクチャの概要】
// ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
// │  Web API層      │────▶│   チャネル      │────▶│  エンジン層     │
// │  (axum)        │◀────│   (mpsc)       │◀────│  (OrderBook)   │
// └─────────────────┘     └─────────────────┘     └─────────────────┘
//
// Refactored into modules:
// - models: データ型 (Order, Trade, Side)
// - db: データベース接続 & 永続化アクター
// - account: 残高管理ロジック
// - orderbook: 板管理ロジック
// - engine: マッチングエンジンアクター
// - simulator: 市場シミュレータ
// =============================================================================

// --- 内部モジュール ---
// モジュールは src/lib.rs に移動し、ライブラリとしてインポートします
// mod models;
// mod db;
// mod account;
// mod orderbook;
// mod engine;
// mod simulator;


// --- 外部クレート（ライブラリ）のインポート ---
use axum::{
    extract::{State, ws::{Message, WebSocket, WebSocketUpgrade}}, // WebSocket機能を追加
    routing::{get, post},     // HTTPメソッドに応じたルーティング
    response::IntoResponse,   // レスポンス変換用トレイト
    Json, Router,             // JSONレスポンスとルーター
};
use rust_decimal::Decimal;    // 固定小数点数
use serde::{Deserialize, Serialize}; 
use std::sync::Arc;           // スレッド間で安全に共有できるスマートポインタ
use std::time::SystemTime;    // UNIXタイムスタンプ取得用
use tokio::sync::{mpsc, oneshot, broadcast}; // broadcastを追加
use tower_http::cors::CorsLayer;  // CORSヘッダーを追加するミドルウェア
use uuid::Uuid;               // ユニークID生成

// --- モジュールからのインポート ---
use rust_matching_engine::models::{Order, Trade, Side, OrderType};
use rust_matching_engine::orderbook::OrderBook;
use rust_matching_engine::account::AccountManager;
use rust_matching_engine::engine::{self, EngineMessage};
use rust_matching_engine::db::{self, DbMessage};
use rust_matching_engine::simulator;


// =============================================================================
// Webサーバーの状態
// =============================================================================

/// APIハンドラーが持つ共有状態
#[derive(Clone)]
struct AppState {
    sender: mpsc::Sender<EngineMessage>,
    db_pool: db::DbPool,      // データベース接続プール
    user_id: Uuid,            // 現在のユーザーID（固定ユーザー）
    broadcast_tx: broadcast::Sender<OrderBook>, // 板情報の配信チャンネル
}

// =============================================================================
// APIハンドラー
// =============================================================================

/// GET /orderbook - 現在の板情報を取得
async fn get_orderbook(State(state): State<Arc<AppState>>) -> Json<OrderBook> {
    let (resp_tx, resp_rx) = oneshot::channel();
    let _ = state.sender.send(EngineMessage::GetOrderBook { respond_to: resp_tx }).await;
    let book = resp_rx.await.unwrap();
    Json(book)
}

/// GET /trades - 取引履歴を取得
async fn get_trades(State(state): State<Arc<AppState>>) -> Json<Vec<Trade>> {
    let (resp_tx, resp_rx) = oneshot::channel();
    let _ = state.sender.send(EngineMessage::GetTrades { respond_to: resp_tx }).await;
    let trades = resp_rx.await.unwrap();
    Json(trades)
}

/// 残高レスポンス用の構造体
#[derive(Serialize)]
struct BalanceResponse {
    usdc_available: String,
    usdc_locked: String,
    bad_available: String,
    bad_locked: String,
}

/// GET /balance - ユーザーの残高を取得
async fn get_balance(State(state): State<Arc<AppState>>) -> Json<BalanceResponse> {
    let balances = db::get_balances(&state.db_pool, state.user_id)
        .await
        .unwrap_or_default();

    let mut response = BalanceResponse {
        usdc_available: "0".to_string(),
        usdc_locked: "0".to_string(),
        bad_available: "0".to_string(),
        bad_locked: "0".to_string(),
    };

    for balance in balances {
        match balance.asset.as_str() {
            "USDC" => {
                response.usdc_available = balance.available.to_string();
                response.usdc_locked = balance.locked.to_string();
            }
            "BAD" => {
                response.bad_available = balance.available.to_string();
                response.bad_locked = balance.locked.to_string();
            }
            _ => {}
        }
    }

    Json(response)
}

/// 新規注文APIのリクエストボディ
#[derive(Deserialize)]
struct CreateOrderPayload {
    #[serde(with = "rust_decimal::serde::str")] // JSONから文字列として受け取る
    price: Decimal,
    quantity: u64,
    side: Side,
    #[serde(default = "default_order_type")]
    order_type: OrderType,
}

/// POST /order - 新規注文を作成
async fn create_order(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateOrderPayload>,
) -> Json<Vec<Trade>> {
    // 注文IDを生成
    let new_order = Order {
        id: (SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 10000000) as u64,
        price: payload.price, // 成行の場合は0などの値が入ってくる想定
        quantity: payload.quantity,
        side: payload.side,
        user_id: Some(state.user_id), // 注文者のIDを設定
        order_type: payload.order_type,
    };

    let (resp_tx, resp_rx) = oneshot::channel();
    
    // エンジンに注文処理を依頼
    let _ = state.sender.send(EngineMessage::PlaceOrder { 
        order: new_order, 
        respond_to: resp_tx 
    }).await;

    // 約定結果を受け取って返す
    let new_trades = resp_rx.await.unwrap();
    Json(new_trades)
}

/// DELETE /order/:id - 注文をキャンセル
async fn cancel_order(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(order_id): axum::extract::Path<u64>,
) -> impl axum::response::IntoResponse {
    let (resp_tx, resp_rx) = oneshot::channel();
    
    // エンジンにキャンセルを依頼
    let _ = state.sender.send(EngineMessage::CancelOrder { 
        order_id, 
        user_id: state.user_id, // 自分の注文しかキャンセルできない
        respond_to: resp_tx 
    }).await;

    // 結果待機
    match resp_rx.await {
        Ok(Some(order)) => {
            // キャンセル成功: 削除された注文を返す
            axum::response::Json(order).into_response()
        },
        Ok(None) => {
            // 注文が見つからない (404 Not Found)
            axum::http::StatusCode::NOT_FOUND.into_response()
        },
        Err(_) => {
            // エンジンとの通信エラー (500)
            axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn default_order_type() -> OrderType {
    OrderType::Limit
}

// =============================================================================
// メイン関数
// =============================================================================

#[tokio::main]
async fn main() {
    // =========================================================================
    // Step 0: データベースを初期化
    // =========================================================================
    let (db_pool, user_id) = db::init_database("data.db")
        .await
        .expect("データベースの初期化に失敗しました");

    // =========================================================================
    // Step 1: データをメモリにロード (AccountManagerの初期化)
    // =========================================================================
    let mut account_manager = AccountManager::new();
    let initial_balances = db::get_balances(&db_pool, user_id).await.unwrap_or_default();
    
    for b in &initial_balances {
        account_manager.load_balance(b.user_id, &b.asset, b.available, b.locked);
    }
    println!("✅ 残高ロード完了: {} 件", initial_balances.len());

    // =========================================================================
    // Step 2: DB Writer Actor（永続化タスク）を起動
    // =========================================================================
    let (db_tx, db_rx) = mpsc::channel::<DbMessage>(10000);
    let db_pool_for_writer = db_pool.clone();
    
    tokio::spawn(async move {
        db::run_db_writer(db_rx, db_pool_for_writer).await;
    });

    // =========================================================================
    // Step 3: Engine Actor（マッチングエンジン）を起動
    // =========================================================================
    let (tx, rx) = mpsc::channel::<EngineMessage>(10000);
    // 板情報配信用のbroadcastチャネル（容量10000）- Lag対策で増やす
    let (broadcast_tx, _) = broadcast::channel::<OrderBook>(10000);
    
    let engine_db_tx = db_tx.clone();
    let engine_broadcast_tx = broadcast_tx.clone();

    // engine::run_matching_engine は async fn なので await が必要だが、
    // ここでは spawn するので async move ブロック内で呼び出す
    tokio::spawn(async move {
        engine::run_matching_engine(rx, engine_db_tx, account_manager, engine_broadcast_tx).await;
    });

    // =========================================================================
    // Step 4: 市場シミュレータを起動
    // =========================================================================
    let sim_sender = tx.clone();
    tokio::spawn(async move {
        simulator::run_market_simulator(sim_sender).await;
    });

    // =========================================================================
    // Step 5: Webサーバーを起動
    // =========================================================================
    let state = Arc::new(AppState {
        sender: tx.clone(),     // チャネルの送信側をクローン
        db_pool: db_pool.clone(), // DBプール
        user_id,                // デフォルトユーザーID
        broadcast_tx: broadcast_tx.clone(), // broadcastチャネル
    });

    // ルーターを構築
    let app = Router::new()
        .route("/orderbook", get(get_orderbook)) // GET /orderbook
        .route("/trades", get(get_trades))       // GET /trades  
        .route("/order", post(create_order))     // POST /order
        .route("/order/{id}", axum::routing::delete(cancel_order)) // DELETE /order/{id}
        .route("/balance", get(get_balance))     // GET /balance
        .route("/ws", get(ws_handler))           // WebSocket
        .layer(CorsLayer::permissive())          // CORS許可（開発用に全許可）
        .with_state(state.clone());              // ハンドラーに状態を渡す

    println!("サーバー起動中: http://localhost:8000");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
/// WebSocketハンドラ
/// クライアントからの接続要求を受け入れ、WebSocket接続にアップグレードする
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl axum::response::IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// WebSocket接続の実体
/// 板情報(OrderBook)の更新をリアルタイムにクライアントへ送信する
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // broadcastチャネルを購読（新しい受信機を作成）
    let mut rx = state.broadcast_tx.subscribe();

    loop {
        tokio::select! {
            // 1. 新しい板情報が配信されたら、クライアントに送信
            result = rx.recv() => {
                match result {
                    Ok(orderbook) => {
                        // JSONにシリアライズ
                        if let Ok(json_text) = serde_json::to_string(&orderbook) {
                            // 送信（エラーならループを抜けて切断扱い）
                            if socket.send(Message::Text(json_text.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        // 受信が遅れている場合はスキップして継続（切断しない）
                        eprintln!("Broadcast channel lagged by {}, skipping...", count);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        eprintln!("Broadcast channel closed");
                        break;
                    }
                }
            }
            // 2. クライアントからのメッセージ（切断検知など）
            // これがないと、クライアントが切断してもループが止まらずリソースリークする可能性がある
            msg = socket.recv() => {
                match msg {
                    Some(Ok(_)) => {
                        // クライアントからのメッセージは無視（今回は一方通行）
                        // 必要ならPing/Pong対応などをここに入れる
                    }
                    Some(Err(_)) | None => {
                        // エラーまたは切断（None）
                        break; 
                    }
                }
            }
        }
    }
}
