use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    // 注文の所有者（シミュレータの場合はNone）
    pub user_id: Option<Uuid>, 
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
