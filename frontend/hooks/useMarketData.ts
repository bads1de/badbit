import { useState, useEffect } from "react";
import { Trade } from "@/types";

export interface MarketStats {
  currentPrice: number;
  priceChange: number;
  priceChangePercent: number;
  volume24h: number;
  startPrice: number;
}

export function useMarketData() {
  const [trades, setTrades] = useState<Trade[]>([]);
  const [marketStats, setMarketStats] = useState<MarketStats>({
    currentPrice: 0,
    priceChange: 0,
    priceChangePercent: 0,
    volume24h: 0,
    startPrice: 0,
  });

  useEffect(() => {
    const fetchTrades = async () => {
      try {
        const tradesRes = await fetch("http://localhost:8000/trades");

        if (tradesRes.ok) {
          const newTrades = await tradesRes.json();
          const sortedTrades = [...newTrades].reverse();
          setTrades(sortedTrades);

          // Calculate Market Stats
          const currentPrice =
            sortedTrades.length > 0 ? parseFloat(sortedTrades[0].price) : 0;
          const now = Date.now();
          const oneDayAgo = now - 24 * 60 * 60 * 1000;
          const recentTrades = sortedTrades.filter(
            (t: Trade) => t.timestamp > oneDayAgo,
          );
          const startPrice =
            recentTrades.length > 0
              ? parseFloat(recentTrades[recentTrades.length - 1].price)
              : currentPrice;
          const priceChange = currentPrice - startPrice;
          const priceChangePercent =
            startPrice > 0 ? (priceChange / startPrice) * 100 : 0;
          const volume24h = recentTrades.reduce(
            (acc: number, t: Trade) => acc + parseFloat(t.price) * t.quantity,
            0,
          );

          setMarketStats({
            currentPrice,
            priceChange,
            priceChangePercent,
            volume24h,
            startPrice,
          });
        }
      } catch (err) {
        console.error("Trades Sync error:", err);
      }
    };

    fetchTrades();
    const interval = setInterval(fetchTrades, 1000);
    return () => clearInterval(interval);
  }, []);

  return { trades, marketStats };
}
