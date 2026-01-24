
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
                    Err(e) => {
                        eprintln!("Broadcast receive error: {}", e);
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
