use tokio::sync::{mpsc, oneshot};
use rand::Rng;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use crate::engine::EngineMessage;
use crate::models::{Order, Side};

/// 市場シミュレータを起動
/// 
/// 実際の取引参加者をシミュレートして、リアルな板を作ります。
/// 10ミリ秒ごとにランダムな注文を生成します。
pub async fn run_market_simulator(sim_sender: mpsc::Sender<EngineMessage>) {
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
        // エンジンが停止していたらシミュレータも終了
        let book = match resp_rx.await {
            Ok(b) => b,
            Err(_) => break, 
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
                // range 0.01..1.5
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
            user_id: None, // シミュレータの注文は所有者なし
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
}
