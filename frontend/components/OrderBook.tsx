"use client";

import { OrderBook as OrderBookType } from "@/types";

interface Props {
  data: OrderBookType;
}

export default function OrderBook({ data }: Props) {
  // Sort Asks: Lowest Price (Best Ask) at the bottom
  const sortedAsks = Object.entries(data.asks)
    .sort((a, b) => parseFloat(b[0]) - parseFloat(a[0])) // Descending (High -> Low)
    .slice(-7); // Take the last 7 (which are the lowest prices)

  // Sort Bids: Highest Price (Best Bid) at the top
  const sortedBids = Object.entries(data.bids)
    .sort((a, b) => parseFloat(b[0]) - parseFloat(a[0])) // Descending (High -> Low)
    .slice(0, 7);

  // Calculate sizes and find max size for bars
  const askSizes = sortedAsks.map(([, orders]) =>
    orders.reduce((acc, o) => acc + o.quantity, 0),
  );
  const bidSizes = sortedBids.map(([, orders]) =>
    orders.reduce((acc, o) => acc + o.quantity, 0),
  );
  const maxSize = Math.max(...askSizes, ...bidSizes, 1);

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
      <div className="grid grid-cols-[1fr_1fr_1fr] gap-1 px-3 py-1.5 text-zinc-500 border-b border-white/5 bg-[#13141b]">
        <span className="truncate">Price</span>
        <span className="text-right truncate">Size</span>
        <span className="text-right truncate">Total</span>
      </div>

      {/* Asks (Sell) - Rendered from Top (High) to Bottom (Low/Best) */}
      <div className="flex-1 overflow-hidden relative">
        <div className="absolute inset-0 flex flex-col justify-end">
          {sortedAsks.map(([price], index) => {
            const size = askSizes[index];
            // Accumulate from bottom (best ask) up to current index
            // sortedAsks is High -> Low. Best is at the end.
            // So we sum from index to end.
            const total = askSizes
              .slice(index)
              .reduce((acc, val) => acc + val, 0);

            const barWidth = (size / maxSize) * 100;

            return (
              <div
                key={price}
                className="grid grid-cols-[1fr_1fr_1fr] gap-1 px-3 py-0.5 hover:bg-white/5 cursor-pointer relative"
              >
                {/* Depth Bar */}
                <div
                  className="absolute top-0 bottom-0 right-0 bg-[#ff5353]/15 transition-all duration-200"
                  style={{ width: `${barWidth}%` }}
                />

                <span className="text-[#ff5353] z-10 relative truncate">
                  {price}
                </span>
                <span className="text-right text-zinc-300 z-10 relative truncate">
                  {size.toLocaleString()}
                </span>
                <span className="text-right text-zinc-500 z-10 relative truncate">
                  {total.toLocaleString()}
                </span>
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
        {sortedBids.map(([price], index) => {
          const size = bidSizes[index];
          // Accumulate from top (best bid) down to current index
          // sortedBids is High -> Low. Best is at start (index 0).
          // So we sum from 0 to index + 1
          const total = bidSizes
            .slice(0, index + 1)
            .reduce((acc, val) => acc + val, 0);

          const barWidth = (size / maxSize) * 100;

          return (
            <div
              key={price}
              className="grid grid-cols-[1fr_1fr_1fr] gap-1 px-3 py-0.5 hover:bg-white/5 cursor-pointer relative"
            >
              {/* Depth Bar */}
              <div
                className="absolute top-0 bottom-0 right-0 bg-[#26E8A6]/15 transition-all duration-200"
                style={{ width: `${barWidth}%` }}
              />

              <span className="text-[#26E8A6] z-10 relative truncate">
                {price}
              </span>
              <span className="text-right text-zinc-300 z-10 relative truncate">
                {size.toLocaleString()}
              </span>
              <span className="text-right text-zinc-500 z-10 relative truncate">
                {total.toLocaleString()}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
