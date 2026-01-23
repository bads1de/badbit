use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use ordered_float::OrderedFloat;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tower_http::cors::CorsLayer;

// --- Matching Engine Logic ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: u64,
    pub price: f64,
    pub quantity: u64,
    pub side: Side,
}

#[derive(Debug, Serialize, Clone)]
pub struct Trade {
    pub maker_id: u64,
    pub taker_id: u64,
    pub price: f64,
    pub quantity: u64,
    pub timestamp: u128,
}

#[derive(Debug, Clone)]
pub struct OrderBook {
    pub bids: BTreeMap<OrderedFloat<f64>, VecDeque<Order>>,
    pub asks: BTreeMap<OrderedFloat<f64>, VecDeque<Order>>,
}

// Custom serialization for OrderBook to convert OrderedFloat keys to strings
impl Serialize for OrderBook {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("OrderBook", 2)?;

        // Convert bids BTreeMap<OrderedFloat<f64>, ...> to BTreeMap<String, ...>
        let bids: BTreeMap<String, &VecDeque<Order>> = self
            .bids
            .iter()
            .map(|(k, v)| (format!("{:.3}", k.0), v))
            .collect();
        state.serialize_field("bids", &bids)?;

        // Convert asks BTreeMap<OrderedFloat<f64>, ...> to BTreeMap<String, ...>
        let asks: BTreeMap<String, &VecDeque<Order>> = self
            .asks
            .iter()
            .map(|(k, v)| (format!("{:.3}", k.0), v))
            .collect();
        state.serialize_field("asks", &asks)?;

        state.end()
    }
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

        let taker_price = OrderedFloat(taker_order.price);

        match taker_order.side {
            Side::Buy => {
                while taker_order.quantity > 0 {
                    let first_price = match self.asks.keys().next() {
                        Some(&p) if p <= taker_price => p,
                        _ => break,
                    };

                    let orders_at_price = self.asks.get_mut(&first_price).unwrap();
                    while taker_order.quantity > 0 && !orders_at_price.is_empty() {
                        let mut maker_order = orders_at_price.pop_front().unwrap();
                        let match_quantity =
                            std::cmp::min(taker_order.quantity, maker_order.quantity);

                        trades.push(Trade {
                            maker_id: maker_order.id,
                            taker_id: taker_order.id,
                            price: first_price.0,
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
                        .entry(taker_price)
                        .or_insert_with(VecDeque::new)
                        .push_back(taker_order);
                }
            }
            Side::Sell => {
                while taker_order.quantity > 0 {
                    let first_price = match self.bids.keys().next_back() {
                        Some(&p) if p >= taker_price => p,
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
                            price: first_price.0,
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
                        .entry(taker_price)
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
    price: f64,
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
            .as_millis()
            % 10000000) as u64,
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

    // --- Advanced Market Simulator (Maker-Taker Strategy) ---
    let sim_state = state.clone();
    tokio::spawn(async move {
        // High frequency loop
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(10));
        let mut id_counter: u64 = 2000000;
        let mut base_price: f64 = 100.0; // Dynamic reference price

        loop {
            interval.tick().await;
            let mut rng = rand::rng();
            id_counter += 1;

            let mut book = sim_state.orderbook.lock().unwrap();
            let mut trades_store = sim_state.trades.lock().unwrap();

            // 1. Analyze current market state
            // Use OrderedFloat keys properly
            let best_bid = book.bids.keys().next_back().map(|k| k.0).unwrap_or(base_price - 0.5);
            let best_ask = book.asks.keys().next().map(|k| k.0).unwrap_or(base_price + 0.5);
            let mid_price = (best_bid + best_ask) / 2.0;
            
            // Slowly drift the base price
            if rng.random_bool(0.01) {
                base_price = mid_price;
            }

            // 2. Decide Strategy: Maker (Add Liquidity) or Taker (Take Liquidity)
            // 90% Maker (Thicken the book), 10% Taker (Execute trades)
            let is_taker = rng.random_bool(0.10); 

            let (price, quantity, side) = if is_taker {
                // --- TAKER (Market Order) ---
                // Aggressively cross the spread to ensure execution
                let side = if rng.random_bool(0.5) { Side::Buy } else { Side::Sell };
                let price = match side {
                    Side::Buy => best_ask + 0.1,  // Buy higher than ask
                    Side::Sell => best_bid - 0.1, // Sell lower than bid
                };
                // Taker trades are smaller/faster chunks
                let qty = rng.random_range(5..50);
                (price, qty, side)
            } else {
                // --- MAKER (Limit Order) ---
                // Place orders AROUND the spread but not crossing it, to build depth
                let side = if rng.random_bool(0.5) { Side::Buy } else { Side::Sell };
                let spread_offset = rng.random_range(0.01..2.0); // Distance from mid price
                
                let price = match side {
                    Side::Buy => (best_bid - spread_offset).max(0.1), // Below best bid
                    Side::Sell => (best_ask + spread_offset),         // Above best ask
                };
                
                // Round to 3 decimals
                let price = (price * 1000.0).round() / 1000.0;
                // Maker orders are larger blocks
                let qty = rng.random_range(50..500); 
                (price, qty, side)
            };

            let new_order = Order {
                id: id_counter,
                price,
                quantity,
                side,
            };

            let new_trades = book.process_order(new_order);
            trades_store.extend(new_trades);
            
            // Cleanup: Keep book from growing infinitely in memory
            // Keep top 500 levels for each side
            if book.bids.len() > 500 {
                let keys_to_remove: Vec<_> = book.bids.keys().take(10).cloned().collect();
                for k in keys_to_remove { book.bids.remove(&k); }
            }
            if book.asks.len() > 500 {
                let keys_to_remove: Vec<_> = book.asks.keys().rev().take(10).cloned().collect();
                for k in keys_to_remove { book.asks.remove(&k); }
            }
        }
    });

    println!("Server running on http://localhost:8000");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}