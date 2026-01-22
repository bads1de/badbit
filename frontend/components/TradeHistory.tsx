"use client";

import { Trade } from "@/types";

interface Props {
  trades: Trade[];
}

export default function TradeHistory({ trades }: Props) {
  return (
    <div className="flex flex-col h-full bg-[#13141b] text-xs font-mono">
      <div className="grid grid-cols-3 px-2 py-1.5 text-zinc-500 border-b border-white/5">
        <span>Price</span>
        <span className="text-right">Size</span>
        <span className="text-right">Time</span>
      </div>
      <div className="flex-1 overflow-y-auto custom-scrollbar">
        {trades.map((trade, i) => {
          const date = new Date(Number(trade.timestamp));
          const timeStr = date.toLocaleTimeString([], {
            hour12: false,
            hour: "2-digit",
            minute: "2-digit",
            second: "2-digit",
          });
          // Mock side logic logic since we don't have it in Trade yet clearly
          const isBuy = trade.maker_id < trade.taker_id;

          return (
            <div
              key={i}
              className="grid grid-cols-3 px-2 py-0.5 hover:bg-white/5 cursor-pointer"
            >
              <span
                className={`${isBuy ? "text-[#26E8A6]" : "text-[#ff5353]"}`}
              >
                {trade.price.toFixed(2)}
              </span>
              <span className="text-right text-zinc-300">
                {trade.quantity.toLocaleString()}
              </span>
              <span className="text-right text-zinc-500">{timeStr}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
