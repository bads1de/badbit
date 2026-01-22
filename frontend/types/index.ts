export type Side = "Buy" | "Sell";

export interface Order {
  id: number;
  price: number;
  quantity: number;
  side: Side;
}

export interface OrderBook {
  bids: Record<number, Order[]>;
  asks: Record<number, Order[]>;
}

export interface Trade {
  maker_id: number;
  taker_id: number;
  price: number;
  quantity: number;
  timestamp: number;
}
