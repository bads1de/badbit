export type Side = "Buy" | "Sell";

export interface Order {
  id: number;
  price: string; // Decimal string
  quantity: number;
  side: Side;
  user_id?: string;
}

export interface OrderBook {
  bids: Record<string, Order[]>;
  asks: Record<string, Order[]>;
}

export interface Trade {
  maker_id: number;
  taker_id: number;
  price: string;
  quantity: number;
  timestamp: number;
}

export interface BalanceResponse {
  usdc_available: string;
  usdc_locked: string;
  bad_available: string;
  bad_locked: string;
}
