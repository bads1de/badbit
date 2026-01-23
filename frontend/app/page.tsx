"use client";

import { useEffect, useState } from "react";
import OrderEntry from "@/components/OrderEntry";
import OrderBook from "@/components/OrderBook";
import TradeHistory from "@/components/TradeHistory";
import PriceChart from "@/components/PriceChart";
import { OrderBook as OrderBookType, Trade } from "@/types";
import { Settings, ChevronDown, CheckCircle2 } from "lucide-react";

export default function Home() {
  const [orderBook, setOrderBook] = useState<OrderBookType>({
    bids: {},
    asks: {},
  });
  const [trades, setTrades] = useState<Trade[]>([]);
  const [activeTab, setActiveTab] = useState<"book" | "trades">("book");
  const [marketStats, setMarketStats] = useState({
    currentPrice: 0,
    priceChange: 0,
    priceChangePercent: 0,
    volume24h: 0,
    startPrice: 0,
  });

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [obRes, tradesRes] = await Promise.all([
          fetch("http://localhost:8000/orderbook"),
          fetch("http://localhost:8000/trades"),
        ]);

        if (obRes.ok) setOrderBook(await obRes.json());

        if (tradesRes.ok) {
          const newTrades = await tradesRes.json();
          const sortedTrades = [...newTrades].reverse();
          setTrades(sortedTrades);

          // Calculate Market Stats
          // 注意: priceはDecimal（文字列）で来るのでparseFloatで変換
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
        console.error("Sync error:", err);
      }
    };

    const interval = setInterval(fetchData, 800);
    return () => clearInterval(interval);
  }, []);

  const chartTrades = [...trades].reverse().map((t) => ({
    price: parseFloat(t.price), // Decimal文字列を数値に変換
    volume: t.quantity,
    timestamp: Number(t.timestamp),
  }));

  const {
    currentPrice,
    priceChange,
    priceChangePercent,
    volume24h,
    startPrice,
  } = marketStats;

  return (
    <main className="h-screen bg-[#0b0c10] text-[#c5c6cc] font-sans flex flex-col overflow-hidden selection:bg-[#26E8A6]/30">
      {/* Header */}
      <nav className="h-12 border-b border-white/5 bg-[#13141b] flex items-center justify-between px-4 shrink-0">
        <div className="flex items-center gap-6">
          <div className="flex items-center gap-2">
            <div className="w-6 h-6 bg-[#26E8A6] rounded-full flex items-center justify-center">
              <span className="text-black font-black text-xs">B</span>
            </div>
            <span className="font-bold text-white tracking-tight">badbit</span>
          </div>

          <div className="h-6 w-px bg-white/10" />

          <div className="flex items-center gap-4 text-xs">
            <div className="flex items-center gap-1 text-white font-bold cursor-pointer hover:bg-white/5 px-2 py-1 rounded">
              BAD/USDC{" "}
              <span className="bg-[#26E8A6] text-black text-[9px] px-1 rounded ml-1">
                Spot
              </span>
              <ChevronDown className="w-3 h-3 text-zinc-500" />
            </div>
            <div className="flex flex-col">
              <span className="text-[10px] text-zinc-500">Price</span>
              <span
                className={`font-mono text-sm ${currentPrice >= startPrice ? "text-[#26E8A6]" : "text-[#ff5353]"}`}
              >
                {currentPrice.toFixed(2)}
              </span>
            </div>
            <div className="flex flex-col">
              <span className="text-[10px] text-zinc-500">24H Change</span>
              <span
                className={`font-mono ${priceChange >= 0 ? "text-[#26E8A6]" : "text-[#ff5353]"}`}
              >
                {priceChange.toFixed(2)} / {priceChangePercent.toFixed(2)}%
              </span>
            </div>
            <div className="flex flex-col">
              <span className="text-[10px] text-zinc-500">24H Volume</span>
              <span className="text-zinc-300 font-mono">
                {volume24h.toLocaleString()} USDC
              </span>
            </div>
          </div>
        </div>

        <div className="flex items-center gap-4">
          <button className="text-xs bg-[#26E8A6] text-black font-bold px-4 py-1.5 rounded hover:bg-[#20c990] transition-colors">
            Connect Wallet
          </button>
          <Settings className="w-4 h-4 text-zinc-500 cursor-pointer hover:text-white" />
        </div>
      </nav>

      <div className="flex-1 flex min-h-0">
        {/* Main Content (Left) */}
        <div className="flex-1 flex flex-col min-w-0">
          {/* Chart Section */}
          <div className="flex-1 bg-[#13141b] relative flex flex-col min-h-0">
            {/* Internal Toolbar */}
            <div className="h-10 border-b border-white/5 flex items-center px-4 gap-4 text-xs font-bold text-zinc-500">
              <div className="flex gap-1">
                <span className="hover:text-white cursor-pointer px-1">1m</span>
                <span className="text-white bg-white/10 px-1 rounded cursor-pointer">
                  5m
                </span>
                <span className="hover:text-white cursor-pointer px-1">1h</span>
                <span className="hover:text-white cursor-pointer px-1">1D</span>
              </div>
              <div className="w-px h-4 bg-white/10" />
              <span className="hover:text-white cursor-pointer">
                Indicators
              </span>
            </div>

            {/* Chart Container */}
            <div className="flex-1 min-h-0 w-full">
              <PriceChart trades={chartTrades} />
            </div>
          </div>

          {/* Bottom Panel (Balances/Orders) */}
          <div className="h-62.5 border-t border-white/5 bg-[#13141b] flex flex-col">
            <div className="h-9 border-b border-white/5 flex items-center px-4 gap-6 text-xs font-bold text-zinc-500">
              <span className="text-white border-b-2 border-[#26E8A6] h-full flex items-center px-1 cursor-pointer">
                Positions
              </span>
              <span className="hover:text-white cursor-pointer h-full flex items-center px-1">
                Open Orders (0)
              </span>
              <span className="hover:text-white cursor-pointer h-full flex items-center px-1">
                Twap
              </span>
              <span className="hover:text-white cursor-pointer h-full flex items-center px-1">
                Trade History
              </span>
              <span className="hover:text-white cursor-pointer h-full flex items-center px-1">
                Funding History
              </span>
            </div>
            <div className="flex-1 flex items-center justify-center text-zinc-600 text-xs">
              <div className="flex flex-col items-center gap-2">
                <CheckCircle2 className="w-8 h-8 opacity-20" />
                <span>No open positions</span>
              </div>
            </div>
          </div>
        </div>

        {/* Right Sidebar (OrderBook / Trades / Entry) */}
        <div className="w-[320px] bg-[#13141b] border-l border-white/5 flex flex-col z-10 shrink-0">
          {/* Tabs */}
          <div className="flex h-10 border-b border-white/5">
            <button
              onClick={() => setActiveTab("book")}
              className={`flex-1 text-xs font-bold uppercase transition-colors ${activeTab === "book" ? "text-white border-b-2 border-[#26E8A6]" : "text-zinc-500 hover:text-zinc-300"}`}
            >
              Order Book
            </button>
            <button
              onClick={() => setActiveTab("trades")}
              className={`flex-1 text-xs font-bold uppercase transition-colors ${activeTab === "trades" ? "text-white border-b-2 border-[#26E8A6]" : "text-zinc-500 hover:text-zinc-300"}`}
            >
              Recent Trades
            </button>
          </div>

          {/* Tab Content */}
          <div className="flex-1 min-h-0">
            {activeTab === "book" ? (
              <OrderBook data={orderBook} />
            ) : (
              <TradeHistory trades={trades} />
            )}
          </div>

          {/* Order Entry (Fixed at bottom of sidebar) */}
          <div className="border-t border-white/5">
            <OrderEntry />
          </div>
        </div>
      </div>
    </main>
  );
}
