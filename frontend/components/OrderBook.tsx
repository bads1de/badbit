"use client";

import { OrderBook as OrderBookType } from "@/types";

interface Props {
  data: OrderBookType;
}

export default function OrderBook({ data }: Props) {
  // Sort Asks: Lowest Price (Best Ask) at the bottom
  const sortedAsks = Object.entries(data.asks)
    .sort((a, b) => parseFloat(b[0]) - parseFloat(a[0])) // Descending (High -> Low)
    .slice(-15); // Take the last 15 (which are the lowest prices)

  // Sort Bids: Highest Price (Best Bid) at the top
  const sortedBids = Object.entries(data.bids)
    .sort((a, b) => parseFloat(b[0]) - parseFloat(a[0])) // Descending (High -> Low)
    .slice(0, 15);

  const bestAsk = parseFloat(sortedAsks[sortedAsks.length - 1]?.[0] || "0");
  const bestBid = parseFloat(sortedBids[0]?.[0] || "0");
  const spread = bestAsk && bestBid ? (bestAsk - bestBid).toFixed(3) : "0.000";
  const spreadPercent =
    bestAsk && bestBid
      ? (((bestAsk - bestBid) / bestAsk) * 100).toFixed(3)
      : "0.000";

  // Calculate totals for accumulation visualization

  // Actually, if we map high->low, and we want Best Ask (Low) at bottom, we just render as is.
  // Wait, standard UI:
  // Asks list:
  // High Price
  // ...
  // Low Price (Best Ask)
  // SPREAD
  // High Price (Best Bid)
  // ...
  // Low Price

  // So my sortedAsks is High -> Low. I can just render it.
  // But for accumulation usually we want the accumulation to start from the spread.
  // Accumulating from Best Ask (Low) upwards to High.
  // So I should sort Asks Low -> High to calculate total, then reverse back to render?
  // Let's keep it simple: just list them.

  return (
    <div className="flex flex-col h-full bg-[#13141b] text-xs font-mono">
      {/* Header */}
      <div className="grid grid-cols-3 px-2 py-1.5 text-zinc-500 border-b border-white/5 bg-[#13141b]">
        <span>Price</span>
        <span className="text-right">Size</span>
        <span className="text-right">Total</span>
      </div>

      {/* Asks (Sell) - Rendered from Top (High) to Bottom (Low/Best) */}
      <div className="flex-1 overflow-hidden relative">
        <div className="absolute inset-0 flex flex-col justify-end">
          {sortedAsks.map(([price, orders]) => {
            const size = orders.reduce((acc, o) => acc + o.quantity, 0);
            return (
              <div
                key={price}
                className="grid grid-cols-3 px-2 py-0.5 hover:bg-white/5 cursor-pointer"
              >
                <span className="text-[#ff5353]">{price}</span>
                <span className="text-right text-zinc-300">
                  {size.toLocaleString()}
                </span>
                <span className="text-right text-zinc-500">-</span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Spread Info */}
      <div className="py-1 px-4 flex justify-between items-center bg-white/5 my-0.5">
        <span
          className={`text-md font-bold ${bestAsk > bestBid ? "text-[#ff5353]" : "text-[#26E8A6]"}`}
        >
          {bestAsk || bestBid || "---"}
        </span>
        <div className="flex gap-2 text-[10px] text-zinc-400">
          <span>Spread</span>
          <span>{spread}</span>
          <span>({spreadPercent}%)</span>
        </div>
      </div>

      {/* Bids (Buy) - Rendered from Top (High/Best) to Bottom (Low) */}
      <div className="flex-1 overflow-hidden">
        {sortedBids.map(([price, orders]) => {
          const size = orders.reduce((acc, o) => acc + o.quantity, 0);
          return (
            <div
              key={price}
              className="grid grid-cols-3 px-2 py-0.5 hover:bg-white/5 cursor-pointer"
            >
              <span className="text-[#26E8A6]">{price}</span>
              <span className="text-right text-zinc-300">
                {size.toLocaleString()}
              </span>
              <span className="text-right text-zinc-500">-</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
