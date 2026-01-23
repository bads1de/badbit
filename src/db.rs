// =============================================================================
// データベースモジュール (SQLite)
// =============================================================================
//
// このモジュールは、アプリケーションの永続化層を担当します。
// 
// 【設計思想】
// - マッチングエンジン（メモリ）の速度を落とさないよう、DB操作は非同期で行う
// - 起動時にDBから状態を読み込み、メモリに展開
// - 約定や残高変更は、別のActorが非同期でDBに書き込む
//
// 【なぜSQLiteを選んだか】
// - セットアップ不要（ファイル1つで完結）
// - Rustとの相性が良い（sqlxが優秀）
// - 開発・学習に最適（本番ならPostgreSQLに移行も容易）
// =============================================================================

use rust_decimal::Decimal;
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use uuid::Uuid;

/// データベース接続プール
/// 
/// 複数の接続を効率的に管理し、並行リクエストを捌けるようにする
pub type DbPool = Pool<Sqlite>;

/// ユーザー情報
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub created_at: i64,
}

/// 残高情報
#[derive(Debug, Clone)]
pub struct Balance {
    pub user_id: Uuid,
    pub asset: String,
    pub available: Decimal,
    pub locked: Decimal,
}

/// データベースを初期化する
/// 
/// 1. SQLiteファイルに接続（なければ作成）
/// 2. テーブルを作成（なければ作成）
/// 3. デフォルトユーザーを作成（いなければ作成）
/// 
/// # 引数
/// - db_path: SQLiteファイルのパス（例: "data.db"）
/// 
/// # 戻り値
/// - 接続プールと、デフォルトユーザーのID
pub async fn init_database(db_path: &str) -> Result<(DbPool, Uuid), sqlx::Error> {
    // 接続プールを作成
    // create_if_missing: ファイルがなければ作成
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&format!("sqlite:{}?mode=rwc", db_path))
        .await?;

    // テーブル作成（IF NOT EXISTSで冪等性を保証）
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            created_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS balances (
            user_id TEXT NOT NULL,
            asset TEXT NOT NULL,
            available TEXT NOT NULL,
            locked TEXT NOT NULL,
            PRIMARY KEY (user_id, asset)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            maker_order_id INTEGER NOT NULL,
            taker_order_id INTEGER NOT NULL,
            price TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            timestamp INTEGER NOT NULL,
            user_id TEXT
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // デフォルトユーザーを取得または作成
    let default_user_id = ensure_default_user(&pool).await?;

    println!("✅ データベース初期化完了: {}", db_path);
    println!("   デフォルトユーザーID: {}", default_user_id);

    Ok((pool, default_user_id))
}

/// デフォルトユーザーを確保する
/// 
/// - 既に存在すれば、そのIDを返す
/// - 存在しなければ、新規作成して初期残高を設定
async fn ensure_default_user(pool: &DbPool) -> Result<Uuid, sqlx::Error> {
    const DEFAULT_USERNAME: &str = "trader";

    // 既存ユーザーを検索
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM users WHERE username = ?"
    )
    .bind(DEFAULT_USERNAME)
    .fetch_optional(pool)
    .await?;

    if let Some((id_str,)) = existing {
        // 既存ユーザーが見つかった
        let user_id = Uuid::parse_str(&id_str).expect("Invalid UUID in database");
        println!("   既存ユーザー発見: {}", DEFAULT_USERNAME);
        return Ok(user_id);
    }

    // 新規ユーザーを作成
    let user_id = Uuid::new_v4();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query("INSERT INTO users (id, username, created_at) VALUES (?, ?, ?)")
        .bind(user_id.to_string())
        .bind(DEFAULT_USERNAME)
        .bind(now)
        .execute(pool)
        .await?;

    // 初期残高を設定: 10,000 USDC
    sqlx::query(
        "INSERT INTO balances (user_id, asset, available, locked) VALUES (?, ?, ?, ?)"
    )
    .bind(user_id.to_string())
    .bind("USDC")
    .bind("10000")  // Decimalは文字列で保存
    .bind("0")
    .execute(pool)
    .await?;

    // BADトークンも初期化（0から開始）
    sqlx::query(
        "INSERT INTO balances (user_id, asset, available, locked) VALUES (?, ?, ?, ?)"
    )
    .bind(user_id.to_string())
    .bind("BAD")
    .bind("0")
    .bind("0")
    .execute(pool)
    .await?;

    println!("   新規ユーザー作成: {} (初期残高: 10,000 USDC)", DEFAULT_USERNAME);

    Ok(user_id)
}

/// ユーザーの残高を取得する
pub async fn get_balances(pool: &DbPool, user_id: Uuid) -> Result<Vec<Balance>, sqlx::Error> {
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT user_id, asset, available, locked FROM balances WHERE user_id = ?"
    )
    .bind(user_id.to_string())
    .fetch_all(pool)
    .await?;

    let balances = rows
        .into_iter()
        .map(|(uid, asset, available, locked)| Balance {
            user_id: Uuid::parse_str(&uid).unwrap(),
            asset,
            available: available.parse().unwrap_or_default(),
            locked: locked.parse().unwrap_or_default(),
        })
        .collect();

    Ok(balances)
}

/// 残高を更新する
pub async fn update_balance(
    pool: &DbPool,
    user_id: Uuid,
    asset: &str,
    available: Decimal,
    locked: Decimal,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE balances SET available = ?, locked = ? WHERE user_id = ? AND asset = ?"
    )
    .bind(available.to_string())
    .bind(locked.to_string())
    .bind(user_id.to_string())
    .bind(asset)
    .execute(pool)
    .await?;

    Ok(())
}

/// 約定をDBに保存する
pub async fn save_trade(
    pool: &DbPool,
    maker_order_id: u64,
    taker_order_id: u64,
    price: Decimal,
    quantity: u64,
    timestamp: u128,
    user_id: Option<Uuid>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO trades (maker_order_id, taker_order_id, price, quantity, timestamp, user_id)
        VALUES (?, ?, ?, ?, ?, ?)
        "#
    )
    .bind(maker_order_id as i64)
    .bind(taker_order_id as i64)
    .bind(price.to_string())
    .bind(quantity as i64)
    .bind(timestamp as i64)
    .bind(user_id.map(|u| u.to_string()))
    .execute(pool)
    .await?;

    Ok(())
}
