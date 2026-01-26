import { useOrderBook } from "./useOrderBook";
import { useCallback } from "react";

export function useMyOrders() {
  const { orderBook } = useOrderBook();

  // Extract orders that have a user_id (implying they belong to the user, not the simulator)
  // Flatten calls: bids and asks are Record<price, Order[]>
  const myBids = Object.values(orderBook.bids)
    .flat()
    .filter((o) => o.user_id);

  const myAsks = Object.values(orderBook.asks)
    .flat()
    .filter((o) => o.user_id);

  const myOrders = [...myBids, ...myAsks].sort((a, b) => b.id - a.id); // Newest first

  const cancelOrder = useCallback(async (orderId: number) => {
    try {
      const res = await fetch(`http://localhost:8000/order/${orderId}`, {
        method: "DELETE",
      });
      if (!res.ok) {
        throw new Error("Failed to cancel order");
      }
      // UI update will happen automatically via WebSocket orderbook update
    } catch (err) {
      console.error("Cancel error:", err);
      alert("Failed to cancel order");
    }
  }, []);

  return { myOrders, cancelOrder };
}
