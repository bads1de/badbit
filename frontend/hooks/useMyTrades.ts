import { useState, useEffect } from "react";
import { Trade } from "@/types";

export function useMyTrades() {
  const [myTrades, setMyTrades] = useState<Trade[]>([]);

  useEffect(() => {
    // 最初のフェッチ
    const fetchTrades = async () => {
      try {
        const res = await fetch("http://localhost:8000/my-trades");
        if (res.ok) {
          const data = await res.json();
          setMyTrades(data);
        }
      } catch (err) {
        console.error("My trades fetch error:", err);
      }
    };

    fetchTrades();

    // 定期ポーリング (3秒ごと)
    // ※本来はWebSocketやSSEでリアルタイム通知を受けるのが理想
    const interval = setInterval(fetchTrades, 3000);

    return () => clearInterval(interval);
  }, []);

  return { myTrades };
}
