use std::collections::{BTreeMap, VecDeque};
use std::time::SystemTime;
use rust_decimal::Decimal;
use serde::Serialize;
use crate::models::{Order, Trade, Side};

/// OrderBook（板）を表す構造体
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
