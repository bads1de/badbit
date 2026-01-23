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
    extract::State,           // ハンドラー関数で共有状態にアクセスするため
    routing::{get, post},     // HTTPメソッドに応じたルーティング
    Json, Router,             // JSONレスポンスとルーター
};
use rust_decimal::Decimal;    // 固定小数点数
use serde::{Deserialize, Serialize}; 
use std::sync::Arc;           // スレッド間で安全に共有できるスマートポインタ
use std::time::SystemTime;    // UNIXタイムスタンプ取得用
use tokio::sync::{mpsc, oneshot}; // 非同期チャネル
use tower_http::cors::CorsLayer;  // CORSヘッダーを追加するミドルウェア
use uuid::Uuid;               // ユニークID生成

// --- モジュールからのインポート ---
// --- モジュールからのインポート ---
use rust_matching_engine::models::{Order, Trade, Side};
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
        price: payload.price,
        quantity: payload.quantity,
        side: payload.side,
        user_id: Some(state.user_id), // 注文者のIDを設定
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
    let engine_db_tx = db_tx.clone();

    // engine::run_matching_engine は async fn なので await が必要だが、
    // ここでは spawn するので async move ブロック内で呼び出す
    tokio::spawn(async move {
        engine::run_matching_engine(rx, engine_db_tx, account_manager).await;
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
    });

    // ルーターを構築
    let app = Router::new()
        .route("/orderbook", get(get_orderbook)) // GET /orderbook
        .route("/trades", get(get_trades))       // GET /trades  
        .route("/order", post(create_order))     // POST /order
        .route("/balance", get(get_balance))     // GET /balance
        .layer(CorsLayer::permissive())          // CORS許可（開発用に全許可）
        .with_state(state.clone());              // ハンドラーに状態を渡す

    println!("サーバー起動中: http://localhost:8000");
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}