"use client";

import { OrderBook as OrderBookType } from "@/types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { TrendingUp, TrendingDown } from "lucide-react";

interface Props {
  data: OrderBookType;
}

export default function OrderBook({ data }: Props) {
  const sortedBids = Object.entries(data.bids)
    .sort((a, b) => parseInt(b[0]) - parseInt(a[0]))
    .slice(0, 15);
  const sortedAsks = Object.entries(data.asks)
    .sort((a, b) => parseInt(a[0]) - parseInt(b[0]))
    .slice(0, 15);

  const bestAsk = parseInt(sortedAsks[0]?.[0] || "0");
  const bestBid = parseInt(sortedBids[0]?.[0] || "0");
  const midPrice = bestAsk && bestBid ? ((bestAsk + bestBid) / 2).toFixed(2) : (bestAsk || bestBid || "---");

  return (
    <Card className="bg-zinc-950 border-zinc-800 shadow-2xl h-full flex flex-col">
      <CardHeader className="pb-4 flex flex-row items-center justify-between space-y-0">
        <CardTitle className="text-sm font-bold uppercase tracking-widest text-zinc-400">Order Book</CardTitle>
        <Badge variant="outline" className="border-zinc-800 text-zinc-500 font-mono text-[10px]">REALTIME</Badge>
      </CardHeader>
      <CardContent className="flex-1 flex flex-col min-h-0">
        <div className="grid grid-cols-2 text-[10px] font-bold text-zinc-600 mb-2 uppercase tracking-tighter">
          <span>Price (USDT)</span>
          <span className="text-right">Size (BAD)</span>
        </div>
        
        {/* Asks (Sell Orders) */}
        <ScrollArea className="flex-1">
          <div className="flex flex-col-reverse">
            {sortedAsks.map(([price, orders]) => (
              <div key={price} className="grid grid-cols-2 text-sm py-[2px] group relative cursor-pointer">
                <div className="absolute inset-0 bg-rose-950/20 origin-right transition-transform scale-x-0 group-hover:scale-x-100" />
                <span className="relative text-rose-500 font-mono pl-1">{price}</span>
                <span className="relative text-right text-zinc-400 font-mono pr-1">
                  {orders.reduce((acc, o) => acc + o.quantity, 0).toLocaleString()}
                </span>
              </div>
            ))}
          </div>
        </ScrollArea>

        {/* Spread / Mid Price */}
        <div className="my-4 py-3 bg-zinc-900/50 border-y border-zinc-800/50 flex items-center justify-center gap-3">
          <div className="flex items-center gap-2">
            <span className="text-2xl font-black text-white tracking-tighter">{midPrice}</span>
            {bestAsk > bestBid ? <TrendingUp className="w-4 h-4 text-emerald-500" /> : <TrendingDown className="w-4 h-4 text-rose-500" />}
          </div>
        </div>

        {/* Bids (Buy Orders) */}
        <ScrollArea className="flex-1">
          <div className="flex flex-col">
            {sortedBids.map(([price, orders]) => (
              <div key={price} className="grid grid-cols-2 text-sm py-[2px] group relative cursor-pointer">
                <div className="absolute inset-0 bg-emerald-950/20 origin-left transition-transform scale-x-0 group-hover:scale-x-100" />
                <span className="relative text-emerald-500 font-mono pl-1">{price}</span>
                <span className="relative text-right text-zinc-400 font-mono pr-1">
                  {orders.reduce((acc, o) => acc + o.quantity, 0).toLocaleString()}
                </span>
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}