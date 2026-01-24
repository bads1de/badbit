use std::collections::HashMap;
use rust_decimal::Decimal;
use uuid::Uuid;
use crate::models::Side;

/// ユーザーごとの残高状態
#[derive(Debug, Clone, Default)]
struct UserBalance {
    available: Decimal,
    locked: Decimal,
}

/// 全ユーザーの残高を管理する
/// 
/// エンジンアクター内で保持され、注文時に高速に残高チェックを行う
#[derive(Debug, Clone, Default)]
pub struct AccountManager {
    // ユーザーID -> { 資産名 -> 残高 }
    balances: HashMap<Uuid, HashMap<String, UserBalance>>,
}

impl AccountManager {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
        }
    }

    /// 初期残高をロードする（起動時用）
    pub fn load_balance(&mut self, user_id: Uuid, asset: &str, available: Decimal, locked: Decimal) {
        let user_balances = self.balances.entry(user_id).or_default();
        user_balances.insert(asset.to_string(), UserBalance { available, locked });
    }

    /// 現在の残高を取得
    pub fn get_balance(&self, user_id: &Uuid, asset: &str) -> (Decimal, Decimal) {
        if let Some(balance) = self.balances.get(user_id).and_then(|b| b.get(asset)) {
            return (balance.available, balance.locked);
        }
        (Decimal::ZERO, Decimal::ZERO)
    }

    /// 注文前の残高チェックとロック（仮押さえ）
    /// 
    /// - 買い注文: (価格 * 数量) 分のUSDCをロック
    /// - 売り注文: 数量分のBADをロック
    pub fn try_lock_balance(&mut self, user_id: &Uuid, side: Side, price: Decimal, quantity: u64) -> Result<(), &'static str> {
        // ロックする量を計算
        let (asset, amount_to_lock) = match side {
            Side::Buy => ("USDC", price * Decimal::from(quantity)),
            Side::Sell => ("BAD", Decimal::from(quantity)),
        };

        let user_balances = self.balances.entry(*user_id).or_default();
        let balance = user_balances.entry(asset.to_string()).or_default();

        if balance.available < amount_to_lock {
            return Err("残高不足"); // Simplified error
        }

        // 残高移動: Available -> Locked
        balance.available -= amount_to_lock;
        balance.locked += amount_to_lock;

        Ok(())
    }

    /// 約定時の残高移動（一番複雑な部分！）
    /// 
    /// 1. 自分のLockedを減らす（注文時にロックした分）
    /// 2. 相手から受け取る資産をAvailableに増やす
    pub fn on_trade_match(&mut self, user_id: &Uuid, side: Side, price: Decimal, quantity: u64) {
        let qty_dec = Decimal::from(quantity);
        let trade_value = price * qty_dec;

        let user_balances = self.balances.entry(*user_id).or_default();

        match side {
            Side::Buy => {
                // 買い手の場合:
                // 1. ロックしていたUSDCを消費（支払う）
                let usdc = user_balances.entry("USDC".to_string()).or_default();
                usdc.locked -= trade_value; // ※注意: ロックした額と一致するはずだが厳密には指値価格との差分返金が必要（今回は省略）
                
                // 2. BADを入手（受け取る）
                let bad = user_balances.entry("BAD".to_string()).or_default();
                bad.available += qty_dec;
            }
            Side::Sell => {
                // 売り手の場合:
                // 1. ロックしていたBADを消費（渡す）
                let bad = user_balances.entry("BAD".to_string()).or_default();
                bad.locked -= qty_dec;

                // 2. USDCを入手（受け取る）
                let usdc = user_balances.entry("USDC".to_string()).or_default();
                usdc.available += trade_value;
            }
        }
    }
}
