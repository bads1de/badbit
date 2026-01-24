import { useState, useEffect } from "react";
import { OrderBook as OrderBookType } from "@/types";

export function useOrderBook() {
  const [orderBook, setOrderBook] = useState<OrderBookType>({
    bids: {},
    asks: {},
  });

  useEffect(() => {
    const ws = new WebSocket("ws://localhost:8000/ws");

    ws.onopen = () => {
      console.log("Connected to OrderBook WebSocket");
    };

    ws.onmessage = (event) => {
      try {
        const newBook: OrderBookType = JSON.parse(event.data);
        setOrderBook(newBook);
      } catch (e) {
        console.error("Failed to parse WebSocket message:", e);
      }
    };

    ws.onerror = (error) => {
      console.error("WebSocket error:", error);
    };

    return () => {
      ws.close();
    };
  }, []);

  return { orderBook };
}
