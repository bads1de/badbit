use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tower_http::cors::CorsLayer;
use rand::Rng;

// --- Matching Engine Logic ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: u64,
    pub price: u64,
    pub quantity: u64,
    pub side: Side,
}

#[derive(Debug, Serialize, Clone)]
pub struct Trade {
    pub maker_id: u64,
    pub taker_id: u64,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: u128,
}

#[derive(Debug, Serialize, Clone)]
pub struct OrderBook {
    pub bids: BTreeMap<u64, VecDeque<Order>>,
    pub asks: BTreeMap<u64, VecDeque<Order>>,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub fn process_order(&mut self, mut taker_order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();

        match taker_order.side {
            Side::Buy => {
                while taker_order.quantity > 0 {
                    let first_price = match self.asks.keys().next() {
                        Some(&p) if p <= taker_order.price => p,
                        _ => break,
                    };

                    let orders_at_price = self.asks.get_mut(&first_price).unwrap();
                    while taker_order.quantity > 0 && !orders_at_price.is_empty() {
                        let mut maker_order = orders_at_price.pop_front().unwrap();
                        let match_quantity = std::cmp::min(taker_order.quantity, maker_order.quantity);

                        trades.push(Trade {
                            maker_id: maker_order.id,
                            taker_id: taker_order.id,
                            price: first_price,
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
                        self.asks.remove(&first_price);
                    }
                }
                if taker_order.quantity > 0 {
                    self.bids
                        .entry(taker_order.price)
                        .or_insert_with(VecDeque::new)
                        .push_back(taker_order);
                }
            }
            Side::Sell => {
                while taker_order.quantity > 0 {
                    let first_price = match self.bids.keys().next_back() {
                        Some(&p) if p >= taker_order.price => p,
                        _ => break,
                    };

                    let orders_at_price = self.bids.get_mut(&first_price).unwrap();
                    while taker_order.quantity > 0 && !orders_at_price.is_empty() {
                        let mut maker_order = orders_at_price.pop_front().unwrap();
                        let match_quantity = std::cmp::min(taker_order.quantity, maker_order.quantity);

                        trades.push(Trade {
                            maker_id: maker_order.id,
                            taker_id: taker_order.id,
                            price: first_price,
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
                if taker_order.quantity > 0 {
                    self.asks
                        .entry(taker_order.price)
                        .or_insert_with(VecDeque::new)
                        .push_back(taker_order);
                }
            }
        }
        trades
    }
}

// --- Web Server State ---

struct AppState {
    orderbook: Mutex<OrderBook>,
    trades: Mutex<Vec<Trade>>,
}

// --- API Handlers ---

async fn get_orderbook(State(state): State<Arc<AppState>>) -> Json<OrderBook> {
    let book = state.orderbook.lock().unwrap();
    Json(book.clone())
}

async fn get_trades(State(state): State<Arc<AppState>>) -> Json<Vec<Trade>> {
    let trades = state.trades.lock().unwrap();
    Json(trades.clone())
}

#[derive(Deserialize)]
struct CreateOrderPayload {
    price: u64,
    quantity: u64,
    side: Side,
}

async fn create_order(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateOrderPayload>,
) -> Json<Vec<Trade>> {
    let mut book = state.orderbook.lock().unwrap();
    let mut trades_store = state.trades.lock().unwrap();

    let new_order = Order {
        id: (SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() % 10000000) as u64,
        price: payload.price,
        quantity: payload.quantity,
        side: payload.side,
    };

    let new_trades = book.process_order(new_order);
    trades_store.extend(new_trades.clone());
    
    Json(new_trades)
}

#[tokio::main]
async fn main() {
    let state = Arc::new(AppState {
        orderbook: Mutex::new(OrderBook::new()),
        trades: Mutex::new(Vec::new()),
    });

    let app = Router::new()
        .route("/orderbook", get(get_orderbook))
        .route("/trades", get(get_trades))
        .route("/order", post(create_order))
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    // --- Random Trade Simulator ---
    let sim_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(1200));
        let mut id_counter = 2000000;
        
        loop {
            interval.tick().await;
            let mut rng = rand::rng();
            id_counter += 1;

            let mut book = sim_state.orderbook.lock().unwrap();
            let mut trades_store = sim_state.trades.lock().unwrap();

            let base_price = 100;
            let offset = rng.random_range(-5..6);
            let final_price = (base_price as i32 + offset).max(1) as u64;
            let quantity = rng.random_range(1..15);
            let side = if rng.random::<bool>() { Side::Buy } else { Side::Sell };

            let new_order = Order {
                id: id_counter,
                price: final_price,
                quantity,
                side,
            };

            let new_trades = book.process_order(new_order);
            trades_store.extend(new_trades);
        }
    });

    println!("Server running on http://localhost:8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}