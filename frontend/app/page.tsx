"use client";

import { useEffect, useState } from "react";
import OrderEntry from "@/components/OrderEntry";
import OrderBook from "@/components/OrderBook";
import TradeHistory from "@/components/TradeHistory";
import PriceChart from "@/components/PriceChart";
import { OrderBook as OrderBookType, Trade } from "@/types";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Activity, Zap, ShieldCheck } from "lucide-react";

export default function Home() {
  const [orderBook, setOrderBook] = useState<OrderBookType>({ bids: {}, asks: {} });
  const [trades, setTrades] = useState<Trade[]>([]);

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
          setTrades([...newTrades].reverse());
        }
      } catch (err) {
        console.error("Sync error:", err);
      }
    };

    const interval = setInterval(fetchData, 800);
    return () => clearInterval(interval);
  }, []);

  const chartTrades = [...trades].reverse().map(t => ({
    price: t.price,
    timestamp: Number(t.timestamp)
  }));

  return (
    <main className="min-h-screen bg-black text-zinc-200 font-sans selection:bg-emerald-500/30">
      {/* Navigation Bar */}
      <nav className="h-14 border-b border-zinc-900 bg-zinc-950/50 backdrop-blur-md sticky top-0 z-50 px-6 flex items-center justify-between">
        <div className="flex items-center gap-8">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 bg-gradient-to-br from-rose-500 to-rose-700 rounded flex items-center justify-center shadow-lg shadow-rose-900/20">
              <Zap className="w-5 h-5 text-white fill-white" />
            </div>
            <span className="text-xl font-black text-white tracking-tighter">
              BAD<span className="text-zinc-500">DEX</span>
            </span>
          </div>
          <Separator orientation="vertical" className="h-6 bg-zinc-800" />
          <div className="flex items-center gap-6">
            <div className="flex flex-col">
              <span className="text-[10px] text-zinc-500 font-bold uppercase leading-none mb-1">Pair</span>
              <span className="text-sm font-bold text-white leading-none">BAD / USDT</span>
            </div>
            <div className="flex flex-col">
              <span className="text-[10px] text-zinc-500 font-bold uppercase leading-none mb-1">Last Price</span>
              <span className="text-sm font-bold text-emerald-500 leading-none">100.42</span>
            </div>
          </div>
        </div>
        <div className="flex items-center gap-4">
          <Badge variant="outline" className="border-emerald-900/50 text-emerald-500 bg-emerald-500/5 px-2 py-1 flex gap-1.5">
            <Activity className="w-3 h-3" />
            SYSTEM ONLINE
          </Badge>
          <div className="flex items-center gap-2 px-3 py-1.5 rounded-full bg-zinc-900 border border-zinc-800">
            <ShieldCheck className="w-4 h-4 text-zinc-500" />
            <span className="text-xs font-bold text-zinc-400">RUST ENGINE 1.92</span>
          </div>
        </div>
      </nav>

      {/* Dashboard Grid */}
      <div className="p-4 grid grid-cols-1 xl:grid-cols-12 gap-4 max-w-[1800px] mx-auto">
        
        {/* Left Column: Order Entry (25%) */}
        <div className="xl:col-span-3 space-y-4">
          <OrderEntry />
          <div className="bg-zinc-950 p-4 rounded-lg border border-zinc-900 space-y-3">
             <h3 className="text-[10px] font-black text-zinc-500 uppercase tracking-widest">Matching Engine Stats</h3>
             <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1">
                   <span className="text-[9px] text-zinc-600 block uppercase">Matching Speed</span>
                   <span className="text-xs font-mono text-zinc-300">{"< 10ms"}</span>
                </div>
                <div className="space-y-1">
                   <span className="text-[9px] text-zinc-600 block uppercase">Priority Logic</span>
                   <span className="text-xs font-mono text-zinc-300">Price-Time</span>
                </div>
             </div>
             <p className="text-[9px] text-zinc-700 leading-relaxed italic">
                Memory-safe, high-concurrency order processing implemented in Rust. Optimized for high-frequency algorithmic trading.
             </p>
          </div>
        </div>

        {/* Middle Column: Chart & Order Book (50%) */}
        <div className="xl:col-span-6 space-y-4">
          {/* Chart Section */}
          <div className="bg-zinc-950 rounded-lg border border-zinc-900 p-4 shadow-2xl relative overflow-hidden group">
            <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-emerald-500 to-rose-500 opacity-20" />
            <div className="flex justify-between items-center mb-4">
              <div className="flex gap-4 items-center">
                <h2 className="text-xs font-black text-zinc-400 uppercase tracking-widest">BAD/USDT Live Chart</h2>
                <Badge className="bg-zinc-900 text-zinc-500 border-none pointer-events-none">1m</Badge>
              </div>
              <div className="flex gap-2">
                <div className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
              </div>
            </div>
            <PriceChart trades={chartTrades} />
          </div>

          {/* Order Book Section */}
          <div className="h-[400px]">
            <OrderBook data={orderBook} />
          </div>
        </div>

        {/* Right Column: History (25%) */}
        <div className="xl:col-span-3 h-[852px]">
          <TradeHistory trades={trades} />
        </div>

      </div>
    </main>
  );
}