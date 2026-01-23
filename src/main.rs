// =============================================================================
// Hyperliquid Bot - 取引マッチングエンジン
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
// - Web API層: HTTPリクエストを受け付ける（複数の接続を同時に処理）
// - チャネル: API層とエンジン層の間のメッセージパッシング
// - エンジン層: 注文の処理とマッチングを行う（シングルスレッドで安全に動作）
//
// 【なぜこの設計なのか？】
// 取引所では「同じデータに複数のリクエストが同時にアクセスする」問題があります。
// 例: AさんとBさんが同時に同じ価格に注文を出す → データの整合性が壊れる可能性
//
// 解決策は2つ:
// 1. Mutex（排他ロック）を使う → シンプルだが、ロック待ちで遅くなる
// 2. Actorパターンを使う → データを1つのタスクだけが触る、他はメッセージを送る
//
// このプログラムは2のActorパターンを採用しています。
// 理由: 高頻度取引ではMutexのロック競合がボトルネックになりやすいため
// =============================================================================

// --- 外部クレート（ライブラリ）のインポート ---
use axum::{
    extract::State,           // ハンドラー関数で共有状態にアクセスするため
    routing::{get, post},     // HTTPメソッドに応じたルーティング
    Json, Router,             // JSONレスポンスとルーター
};
use rand::Rng;                // 乱数生成（シミュレータで使用）
use rust_decimal::Decimal;    // 固定小数点数（お金の計算に必須）
                              // 理由: f64は浮動小数点の誤差がある (0.1 + 0.2 != 0.3)
                              // Decimalは誤差なく正確に10進数を扱える
use rust_decimal_macros::dec; // Decimalリテラルを書くためのマクロ (例: dec!(100.5))
use serde::{Deserialize, Serialize}; // JSON変換のためのシリアライズ/デシリアライズ
use std::collections::{BTreeMap, VecDeque}; // ソート済みマップとキュー
use std::sync::Arc;           // スレッド間で安全に共有できるスマートポインタ
use std::time::SystemTime;    // UNIXタイムスタンプ取得用
use tokio::sync::{mpsc, oneshot}; // 非同期チャネル
                                   // mpsc: 複数送信者→1受信者（Multi-Producer Single-Consumer）
                                   // oneshot: 1回限りの返信用チャネル
use tower_http::cors::CorsLayer;  // CORSヘッダーを追加するミドルウェア

// =============================================================================
// データ構造の定義
// =============================================================================

/// 注文の売買方向を表す列挙型
/// 
/// - Buy: 買い注文（指定価格以下の売り注文があれば約定、なければ板に追加）
/// - Sell: 売り注文（指定価格以上の買い注文があれば約定、なければ板に追加）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

/// 1つの注文を表す構造体
/// 
/// # フィールド
/// - id: 注文を一意に識別するID
/// - price: 希望価格（この価格で取引したい）
/// - quantity: 数量（いくつ欲しいか/売りたいか）
/// - side: 買いか売りか
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: u64,
    #[serde(with = "rust_decimal::serde::str")] // JSONでは文字列として扱う（精度を保つため）
    pub price: Decimal,
    pub quantity: u64,
    pub side: Side,
}

/// 約定（マッチングが成立した取引）を表す構造体
/// 
/// 取引が成立すると、買い手と売り手の注文がマッチして約定が生成されます。
/// 
/// # フィールド
/// - maker_id: 先に板に注文を出していた側のID（流動性を提供した側）
/// - taker_id: 後から来て即座に約定した側のID（流動性を消費した側）
/// - price: 約定価格
/// - quantity: 約定数量
/// - timestamp: 約定時刻（ミリ秒単位のUNIXタイムスタンプ）
#[derive(Debug, Serialize, Clone)]
pub struct Trade {
    pub maker_id: u64,
    pub taker_id: u64,
    #[serde(with = "rust_decimal::serde::str")] // JSONでは文字列として扱う
    pub price: Decimal,
    pub quantity: u64,
    pub timestamp: u128, // u128を使う理由: ミリ秒単位だとu64では2500万年後に溢れる
                          // u128なら事実上無限に使える
}

/// オーダーブック（板）を表す構造体
/// 
/// 取引所の核心部分。すべての未約定注文を価格ごとに管理します。
/// 
/// # フィールド
/// - bids: 買い注文一覧（価格→注文キューのマップ）
/// - asks: 売り注文一覧（価格→注文キューのマップ）
/// 
/// # なぜBTreeMapを使うのか？
/// - 価格順にソートされた状態を維持できる
/// - 最高買値/最安売値を高速に取得できる（イテレータでO(1)）
/// - 挿入・削除はO(log n)だが、HashMapだと毎回ソートが必要になり遅い
/// 
/// # なぜVecDequeを使うのか？
/// - 同じ価格に複数の注文が存在できる
/// - 先入先出（FIFO）で公平に処理するため、キュー構造が適切
/// - 先頭からの取り出しがO(1)（Vecだと先頭削除はO(n)）
#[derive(Debug, Clone)]
pub struct OrderBook {
    // Decimalは既にOrdトレイトを実装しているので、OrderedFloatラッパーは不要！
    // これはDecimalを使う大きなメリットの一つ
    pub bids: BTreeMap<Decimal, VecDeque<Order>>, // 買い板
    pub asks: BTreeMap<Decimal, VecDeque<Order>>, // 売り板
}

/// OrderBook用のカスタムシリアライズ実装
/// 
/// # なぜ手動実装するのか？
/// - Decimalをそのままキーにするとフロントエンドで扱いにくい
/// - 価格を文字列キーとしてJSONに出力したい
/// - 例: { "100.500": [...] } のようなJSON形式にする
impl Serialize for OrderBook {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        
        // 2つのフィールド（bids, asks）を持つ構造体としてシリアライズ
        let mut state = serializer.serialize_struct("OrderBook", 2)?;

        // bidsをシリアライズ: Decimalを文字列キーに変換
        let bids: BTreeMap<String, &VecDeque<Order>> = self
            .bids
            .iter()
            .map(|(k, v)| (k.to_string(), v)) // Decimalはto_string()で正確な文字列に
            .collect();
        state.serialize_field("bids", &bids)?;

        // asksも同様に変換
        let asks: BTreeMap<String, &VecDeque<Order>> = self
            .asks
            .iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        state.serialize_field("asks", &asks)?;

        state.end()
    }
}

/// Defaultトレイトの実装
/// 
/// RustではDefault traitを実装することで:
/// - OrderBook::default() で新しいインスタンスを作れる
/// - 他の型との相互運用性が向上する（Option::unwrap_or_default()など）
impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBook {
    /// 新しい空のオーダーブックを作成
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    /// 注文を処理し、マッチングを行う
    /// 
    /// これが取引所の心臓部。注文が来たら:
    /// 1. マッチ可能な相手注文を探す
    /// 2. 見つかったら約定を生成
    /// 3. 残りがあれば板に追加
    /// 
    /// # 引数
    /// - taker_order: 新しく入ってきた注文（mutなのは数量を減らしていくため）
    /// 
    /// # 戻り値
    /// - 生成された約定のリスト（マッチしなければ空のVec）
    pub fn process_order(&mut self, mut taker_order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        
        // 現在時刻を取得（約定のタイムスタンプ用）
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH) // 1970年1月1日からの経過時間
            .unwrap() // SystemTimeがUNIX_EPOCHより前になることはないのでunwrapは安全
            .as_millis(); // ミリ秒に変換

        // Decimalはそのままキーとして使える（Ordトレイトを持つ）
        let taker_price = taker_order.price;

        match taker_order.side {
            Side::Buy => {
                // ========================================
                // 買い注文の処理
                // ========================================
                // 買い手は「この価格以下で売りたい人」とマッチする
                // つまり、売り板(asks)の安い順に見ていく
                
                // 注文数量がなくなるまでマッチングを続ける
                while taker_order.quantity > 0 {
                    // 最安の売り注文の価格を取得
                    // asks.keys().next() で最小キー（最安値）を取得
                    // BTreeMapは昇順なのでnext()で最小値が得られる
                    let first_price = match self.asks.keys().next() {
                        Some(&p) if p <= taker_price => p, // 買い希望価格以下なら取引可能
                        _ => break, // マッチする売り注文がなければループ終了
                    };

                    // その価格にある注文一覧を取得
                    // unwrap()は安全: 上でkeysから取得したキーなので必ず存在する
                    let orders_at_price = self.asks.get_mut(&first_price).unwrap();
                    
                    // その価格帯の注文を順番に処理
                    while taker_order.quantity > 0 && !orders_at_price.is_empty() {
                        // キューの先頭（最も早く出された注文）を取り出す
                        let mut maker_order = orders_at_price.pop_front().unwrap();
                        
                        // 約定数量 = 両者の数量の小さい方
                        let match_quantity =
                            std::cmp::min(taker_order.quantity, maker_order.quantity);

                        // 約定を記録
                        trades.push(Trade {
                            maker_id: maker_order.id,
                            taker_id: taker_order.id,
                            price: first_price, // Decimalはそのまま使える
                            quantity: match_quantity,
                            timestamp: now,
                        });

                        // 各注文の残数量を更新
                        taker_order.quantity -= match_quantity;
                        maker_order.quantity -= match_quantity;

                        // maker_orderに残りがあれば、キューの先頭に戻す
                        // 理由: まだ約定していない分は次のテイカーに回す
                        if maker_order.quantity > 0 {
                            orders_at_price.push_front(maker_order);
                        }
                    }
                    
                    // この価格帯の注文がすべて約定したらエントリーを削除
                    // 理由: 空のVecDequeを残すとメモリの無駄になる
                    if orders_at_price.is_empty() {
                        self.asks.remove(&first_price);
                    }
                }
                
                // テイカー注文に残りがあれば、買い板に追加
                // これで「指値注文」として板に載る
                if taker_order.quantity > 0 {
                    self.bids
                        .entry(taker_price)           // そのキーのエントリーを取得
                        .or_default()                 // なければデフォルト値（空のVecDeque）を作成
                        .push_back(taker_order);       // キューの末尾に追加
                }
            }
            Side::Sell => {
                // ========================================
                // 売り注文の処理
                // ========================================
                // 売り手は「この価格以上で買いたい人」とマッチする
                // つまり、買い板(bids)の高い順に見ていく
                
                while taker_order.quantity > 0 {
                    // 最高買値を取得
                    // next_back()を使う理由: BTreeMapは昇順なので、最大値は末尾にある
                    let first_price = match self.bids.keys().next_back() {
                        Some(&p) if p >= taker_price => p, // 売り希望価格以上なら取引可能
                        _ => break,
                    };

                    let orders_at_price = self.bids.get_mut(&first_price).unwrap();
                    while taker_order.quantity > 0 && !orders_at_price.is_empty() {
                        let mut maker_order = orders_at_price.pop_front().unwrap();
                        let match_quantity =
                            std::cmp::min(taker_order.quantity, maker_order.quantity);

                        trades.push(Trade {
                            maker_id: maker_order.id,
                            taker_id: taker_order.id,
                            price: first_price, // Decimalはそのまま使える
                            quantity: match_quantity,
                            timestamp: now,
                        });

                        taker_order.quantity -= match_quantity;
                        maker_order.quantity -= match_quantity;

                        if maker_order.quantity > 0 {
                            orders_at_price.push_front(maker_order);
                        }
                    }
                    if orders_at_price.is_empty() {
                        self.bids.remove(&first_price);
                    }
                }
                
                // 残りがあれば売り板に追加
                if taker_order.quantity > 0 {
                    self.asks
                        .entry(taker_price)
                        .or_default()                 // デフォルト値を使う（VecDequeは空のキュー）
                        .push_back(taker_order);
                }
            }
        }
        trades
    }
}

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
enum EngineMessage {
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

// =============================================================================
// Webサーバーの状態
// =============================================================================

/// APIハンドラーが持つ共有状態
/// 
/// # 重要な設計ポイント
/// - OrderBookを直接持たない（Actorパターン）
/// - 代わりにエンジンへのメッセージ送信チャネルだけを持つ
/// 
/// # なぜClone可能にするのか？
/// - axumはハンドラーごとにStateのクローンを渡す
/// - mpsc::SenderはClone可能で、複数の送信者が同じ受信者に送れる
/// - これにより複数のHTTPリクエストが同時にエンジンにメッセージを送れる
#[derive(Clone)]
struct AppState {
    sender: mpsc::Sender<EngineMessage>,
}

// =============================================================================
// APIハンドラー
// =============================================================================
// 
// 各ハンドラーは同じパターンで動作します:
// 1. oneshot チャネルを作成（返信を受け取るため）
// 2. エンジンにメッセージを送信（返信先を含める）
// 3. 結果が返ってくるのを待つ
// 4. JSONで応答

/// GET /orderbook - 現在の板情報を取得
async fn get_orderbook(State(state): State<Arc<AppState>>) -> Json<OrderBook> {
    // 返信用の1回限りのチャネルを作成
    // resp_tx: 送信側（エンジンが使う）
    // resp_rx: 受信側（このハンドラーが使う）
    let (resp_tx, resp_rx) = oneshot::channel();
    
    // エンジンにリクエストを送信
    // _ = で戻り値を無視しているのは、送信エラーはここでは処理しないため
    // （エラーならresp_rx.awaitで検知できる）
    let _ = state.sender.send(EngineMessage::GetOrderBook { respond_to: resp_tx }).await;
    
    // エンジンからの応答を待つ
    // unwrap()の理由: エンジンが応答しないのは致命的エラーなのでパニックでよい
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
    // 現在時刻のミリ秒を10000000で割った余りを使用
    // 理由: 一意性は保証されないが、シンプルで衝突確率は低い
    // 本番環境ではUUIDやシーケンスを使うべき
    let new_order = Order {
        id: (SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % 10000000) as u64,
        price: payload.price,
        quantity: payload.quantity,
        side: payload.side,
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
    // Step 1: メッセージパッシング用のチャネルを作成
    // =========================================================================
    // 
    // mpsc::channel(10000) の意味:
    // - mpsc = Multi-Producer Single-Consumer（複数送信者、1受信者）
    // - 10000 = バッファサイズ（キューに溜められるメッセージ数）
    // 
    // バッファサイズの役割:
    // - エンジンの処理が追いつかなくても、10000件まではキューに溜められる
    // - 10000件を超えると送信側がブロックされる（バックプレッシャー）
    // - これにより、システムがメモリを使い果たすのを防ぐ
    let (tx, mut rx) = mpsc::channel::<EngineMessage>(10000);

    // =========================================================================
    // Step 2: エンジンタスク（アクター）を起動
    // =========================================================================
    // 
    // tokio::spawnで別タスクとして実行される。
    // このタスクだけがOrderBookとTradeHistoryに直接アクセスできる。
    // → ロック不要で安全に並行処理できる理由
    tokio::spawn(async move {
        // --- このタスクだけがデータを所有する ---
        // 注意: tokioのマルチスレッドランタイムでは、.awaitを挟むと
        // 異なるスレッドで再開される可能性がある。しかし、このタスクは
        // 「逐次的に」メッセージを処理するので、同時アクセスは発生しない。
        // OrderBookとtrades_historyは、このタスクだけが所有している。
        // 他のタスクは直接触れない。これが「Actor」の特徴。
        let mut orderbook = OrderBook::new();
        let mut trades_history: Vec<Trade> = Vec::new();

        // メッセージを受け取るたびに処理（無限ループ）
        // rx.recv().await は、メッセージが来るまでこのタスクを休ませる
        // → CPUを消費しないので効率的
        while let Some(msg) = rx.recv().await {
            match msg {
                EngineMessage::PlaceOrder { order, respond_to } => {
                    // 注文をマッチング処理
                    let new_trades = orderbook.process_order(order);
                    // 約定履歴に追加
                    trades_history.extend(new_trades.clone());
                    // 結果を返信（送信側でエラーになっても無視）
                    let _ = respond_to.send(new_trades);
                },
                EngineMessage::GetOrderBook { respond_to } => {
                    // 現在のオーダーブックのクローンを返す
                    // クローンする理由: 所有権を渡すと、次のリクエストで使えなくなる
                    let _ = respond_to.send(orderbook.clone());
                },
                EngineMessage::GetTrades { respond_to } => {
                    // 履歴のクローンを返す
                    let _ = respond_to.send(trades_history.clone());
                }
            }
            
            // =========================================
            // メモリ管理: 古い履歴を定期的に削除
            // =========================================
            // 理由: 履歴が無限に増えるとメモリを食い尽くす
            // 方針: 5000件を超えたら、最新2000件だけ残す
            if trades_history.len() > 5000 {
                let tail = trades_history.len() - 2000;
                // drain(0..tail) で先頭からtail件を削除
                // .drain() はイテレータを返すので、collect()などで消費するか、
                // 単に破棄する（ここでは破棄）
                trades_history.drain(0..tail);
            }
        }
    });

    // =========================================================================
    // Step 3: Webサーバーのセットアップ
    // =========================================================================
    
    // Arc（Atomic Reference Counting）でラップ
    // 理由: 複数のタスク/スレッドで安全に共有するため
    // AppStateは内部にmpsc::Senderだけ持つので、Clone可能
    let state = Arc::new(AppState {
        sender: tx.clone(), // チャネルの送信側をクローン
    });

    // ルーターを構築
    let app = Router::new()
        .route("/orderbook", get(get_orderbook)) // GET /orderbook
        .route("/trades", get(get_trades))       // GET /trades  
        .route("/order", post(create_order))     // POST /order
        .layer(CorsLayer::permissive())          // CORS許可（開発用に全許可）
        .with_state(state.clone());              // ハンドラーに状態を渡す

    // =========================================================================
    // Step 4: 市場シミュレータを起動
    // =========================================================================
    // 
    // 実際の取引参加者をシミュレートして、リアルな板を作ります。
    // 10ミリ秒ごとにランダムな注文を生成します。
    let sim_sender = tx.clone();
    tokio::spawn(async move {
        // 10ミリ秒ごとに発火するタイマー
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(10));
        let mut id_counter: u64 = 2000000; // 注文IDのカウンター
        let mut base_price: Decimal = dec!(100.0);   // 基準価格（価格はこの周辺で動く）

        loop {
            interval.tick().await; // 10ミリ秒待つ
            id_counter += 1;

            // ----------------------------------------------------
            // 現在の板情報を取得
            // ----------------------------------------------------
            // シミュレータがリアルな注文を出すには、現在の最良買値/売値を知る必要がある
            // エンジンに問い合わせて取得
            let (resp_tx, resp_rx) = oneshot::channel();
            let _ = sim_sender.send(EngineMessage::GetOrderBook { respond_to: resp_tx }).await;
            let book = match resp_rx.await {
                Ok(b) => b,
                Err(_) => break, // エンジンが停止していたらシミュレータも終了
            };

            // ----------------------------------------------------
            // 乱数生成器の使用を .await の前に限定する
            // ----------------------------------------------------
            // 
            // 【重要】Rustのasync/await特有の問題
            // 
            // rand::rngが返す乱数生成器は `!Send` （スレッド間で送れない）
            // 理由: 内部でRc（スレッドセーフでない参照カウント）を使っているため
            // 
            // tokio::spawnは、タスクを異なるスレッドで実行する可能性がある。
            // .awaitを挟むと、その前後で異なるスレッドになる可能性がある。
            // 
            // → !Send な値が .await をまたいで生存していると、コンパイルエラーになる
            // 
            // 解決策: ブロック {} を使って、rngのスコープを .await の前に限定する
            // ブロックを抜けるとrngはドロップされるので、.await後には存在しない
            // 
            let (price, quantity, side) = {
                let mut rng = rand::rng();

                // 最良買値と最良売値を取得（なければデフォルト値）
                // Decimalはそのままコピーできる（Copyトレイト実装済み）
                let best_bid = book.bids.keys().next_back().copied().unwrap_or(base_price - dec!(0.5));
                let best_ask = book.asks.keys().next().copied().unwrap_or(base_price + dec!(0.5));
                let mid_price = (best_bid + best_ask) / dec!(2); // 仲値

                // 1%の確率で基準価格を更新（価格のドリフトをシミュレート）
                if rng.random_bool(0.01) {
                    base_price = mid_price;
                }
                
                // 10%の確率でテイカー注文（すぐに約定する注文）
                // 90%はメイカー注文（板に残る注文）
                let is_taker = rng.random_bool(0.10);

                if is_taker {
                    // テイカー: 板の反対側をすぐに約定させる価格で注文
                    let side = if rng.random_bool(0.5) { Side::Buy } else { Side::Sell };
                    let price = match side {
                        Side::Buy => best_ask + dec!(0.1),   // 最安売値より高くして確実に約定
                        Side::Sell => best_bid - dec!(0.1), // 最高買値より安くして確実に約定
                    };
                    let qty = rng.random_range(5..50); // 小さめの数量
                    (price, qty, side)
                } else {
                    // メイカー: スプレッド内に注文を置く
                    let side = if rng.random_bool(0.5) { Side::Buy } else { Side::Sell };
                    // ランダムなオフセットを生成してDecimalに変換
                    let spread_offset_f64: f64 = rng.random_range(0.01..1.5);
                    let spread_offset = Decimal::try_from(spread_offset_f64).unwrap_or(dec!(0.5));
                    let price = match side {
                        Side::Buy => (best_bid - spread_offset).max(dec!(0.1)), // 最良買値より少し下
                        Side::Sell => best_ask + spread_offset,                 // 最良売値より少し上
                    };
                    // 価格を小数点3桁に丸める
                    let price = price.round_dp(3);
                    let qty = rng.random_range(50..500); // 大きめの数量
                    (price, qty, side)
                }
            }; // ← ここでrngがドロップされる

            // 注文オブジェクトを作成
            let new_order = Order {
                id: id_counter,
                price,
                quantity,
                side,
            };

            // エンジンに注文を送信
            // ここに .await があるが、rngはすでにドロップされているので問題なし
            let (done_tx, _done_rx) = oneshot::channel();
            let _ = sim_sender.send(EngineMessage::PlaceOrder { 
                order: new_order, 
                respond_to: done_tx 
            }).await;
            
            // 応答を待たない理由: シミュレータは高速にループし続けたい
            // 約定結果が必要ないので、受信側(_done_rx)は使わずに破棄
        }
    });

    // =========================================================================
    // Step 5: サーバーを起動
    // =========================================================================
    println!("サーバー起動中: http://localhost:8000");
    
    // TCPリスナーをバインド
    // 0.0.0.0 は「すべてのネットワークインターフェース」を意味する
    // → localhost以外からもアクセス可能になる
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    
    // サーバーを起動し、リクエストの処理を開始
    // この行は通常リターンしない（サーバーが停止するまで）
    axum::serve(listener, app).await.unwrap();
}