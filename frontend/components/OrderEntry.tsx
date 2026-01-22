"use client";

import { useState } from "react";
import { Side } from "@/types";

export default function OrderEntry() {
  const [price, setPrice] = useState("");
  const [quantity, setQuantity] = useState("");
  const [side, setSide] = useState<Side>("Buy");
  const [percent, setPercent] = useState(0);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await fetch("http://localhost:8000/order", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          price: parseFloat(price),
          quantity: parseFloat(quantity),
          side,
        }),
      });
      setPrice("");
      setQuantity("");
    } catch (err) {
      console.error("Order failed", err);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[#1b1c25] p-3 text-sm">
      {/* Buy/Sell Toggle */}
      <div className="flex bg-black/40 rounded-lg p-1 mb-4">
        <button
          onClick={() => setSide("Buy")}
          className={`flex-1 py-1.5 rounded-md font-bold text-center transition-colors ${
            side === "Buy"
              ? "bg-[#26E8A6] text-black"
              : "text-zinc-400 hover:text-white"
          }`}
        >
          Buy
        </button>
        <button
          onClick={() => setSide("Sell")}
          className={`flex-1 py-1.5 rounded-md font-bold text-center transition-colors ${
            side === "Sell"
              ? "bg-[#ff5353] text-black"
              : "text-zinc-400 hover:text-white"
          }`}
        >
          Sell
        </button>
      </div>

      <div className="flex justify-between text-xs text-zinc-400 mb-2">
        <span>Available to Trade</span>
        <span className="text-white font-mono">1,402.50 USDC</span>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Order Type (Limit/Market/Stop) - Mock for now */}
        <div className="flex gap-4 text-xs font-bold text-zinc-500 pb-2 border-b border-white/5">
          <span className="text-white cursor-pointer">Limit</span>
          <span className="hover:text-zinc-300 cursor-pointer">Market</span>
          <span className="hover:text-zinc-300 cursor-pointer">Stop</span>
        </div>

        {/* Price Input */}
        <div className="space-y-1">
          <div className="flex justify-between text-xs text-zinc-500">
            <span>Price</span>
            <span>USDC</span>
          </div>
          <div className="relative">
            <input
              type="number"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              className="w-full bg-[#2a2b36] border border-transparent focus:border-zinc-600 rounded px-3 py-2 text-right font-mono text-white outline-none"
              placeholder="0.00"
            />
          </div>
        </div>

        {/* Size Input */}
        <div className="space-y-1">
          <div className="flex justify-between text-xs text-zinc-500">
            <span>Size</span>
            <span>BAD</span>
          </div>
          <div className="relative">
            <input
              type="number"
              value={quantity}
              onChange={(e) => setQuantity(e.target.value)}
              className="w-full bg-[#2a2b36] border border-transparent focus:border-zinc-600 rounded px-3 py-2 text-right font-mono text-white outline-none"
              placeholder="0.00"
            />
            <div className="absolute left-3 top-2 text-zinc-600 text-xs font-mono">
              {/* Optional Decorator */}
            </div>
          </div>
        </div>

        {/* Slider */}
        <div className="py-2">
          <input
            type="range"
            min="0"
            max="100"
            value={percent}
            onChange={(e) => setPercent(Number(e.target.value))}
            className="w-full h-1 bg-zinc-700 rounded-lg appearance-none cursor-pointer accent-[#26E8A6]"
          />
          <div className="flex justify-between text-[10px] text-zinc-500 mt-1 font-mono">
            <span>0%</span>
            <span>25%</span>
            <span>50%</span>
            <span>75%</span>
            <span>100%</span>
          </div>
        </div>

        {/* Info Rows */}
        <div className="space-y-1 text-xs">
          <div className="flex justify-between text-zinc-500">
            <span>Order Value</span>
            <span className="text-zinc-300 font-mono">
              {(parseFloat(price || "0") * parseFloat(quantity || "0")).toFixed(
                2,
              )}{" "}
              USDC
            </span>
          </div>
        </div>

        <button
          type="submit"
          className={`w-full py-3 rounded-lg font-bold text-lg mt-4 shadow-lg ${
            side === "Buy"
              ? "bg-[#26E8A6] hover:bg-[#20c990] text-black shadow-[#26E8A6]/20"
              : "bg-[#ff5353] hover:bg-[#e04848] text-white shadow-[#ff5353]/20"
          }`}
        >
          {side} BAD
        </button>
      </form>
    </div>
  );
}
