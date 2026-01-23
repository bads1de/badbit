"use client";

import { useState, useEffect } from "react";
import { Side, Trade, BalanceResponse } from "@/types";

export default function OrderEntry() {
  const [price, setPrice] = useState("");
  const [quantity, setQuantity] = useState("");
  const [side, setSide] = useState<Side>("Buy");
  const [percent, setPercent] = useState(0);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [lastResult, setLastResult] = useState<{
    type: "success" | "error" | "partial";
    message: string;
    trades: Trade[];
  } | null>(null);

  const [balances, setBalances] = useState<BalanceResponse>({
    usdc_available: "0",
    usdc_locked: "0",
    bad_available: "0",
    bad_locked: "0",
  });

  useEffect(() => {
    const fetchBalance = async () => {
      try {
        const res = await fetch("http://localhost:8000/balance");
        if (res.ok) {
          const data = await res.json();
          setBalances(data);
        }
      } catch (err) {
        console.error("Failed to fetch balance", err);
      }
    };

    fetchBalance();
    const interval = setInterval(fetchBalance, 1000); // 1秒ごとに更新
    return () => clearInterval(interval);
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!price || !quantity) return;

    setIsSubmitting(true);
    setLastResult(null);

    try {
      const res = await fetch("http://localhost:8000/order", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          // Decimalはバックエンドで文字列として受け取る
          price: price,
          quantity: parseInt(quantity),
          side,
        }),
      });

      if (res.ok) {
        const trades: Trade[] = await res.json();
        const totalFilled = trades.reduce((acc, t) => acc + t.quantity, 0);
        const requestedQty = parseInt(quantity);

        if (trades.length === 0) {
          // 約定なし（指値で板に載った）
          setLastResult({
            type: "success",
            message: `📋 指値注文が板に追加されました (${side} ${requestedQty} @ ${price})`,
            trades: [],
          });
        } else if (totalFilled < requestedQty) {
          // 部分約定
          setLastResult({
            type: "partial",
            message: `⚡ 部分約定: ${totalFilled}/${requestedQty} 約定`,
            trades,
          });
        } else {
          // 全約定
          const avgPrice =
            trades.reduce(
              (acc, t) => acc + parseFloat(t.price) * t.quantity,
              0,
            ) / totalFilled;
          setLastResult({
            type: "success",
            message: `✅ 全約定! ${totalFilled} @ 平均 ${avgPrice.toFixed(3)}`,
            trades,
          });
        }

        setPrice("");
        setQuantity("");
        setPercent(0);
      } else {
        setLastResult({
          type: "error",
          message: "❌ 注文失敗: 残高不足の可能性があります",
          trades: [],
        });
      }
    } catch (err) {
      console.error("Order failed", err);
      setLastResult({
        type: "error",
        message: "❌ サーバー接続エラー",
        trades: [],
      });
    } finally {
      setIsSubmitting(false);
      // 5秒後に結果を消す
      setTimeout(() => setLastResult(null), 5000);
    }
  };

  const availableBalance =
    side === "Buy" ? balances.usdc_available : balances.bad_available;
  const balanceAsset = side === "Buy" ? "USDC" : "BAD";

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

      {/* 約定結果のフィードバック表示 */}
      {lastResult && (
        <div
          className={`mb-3 p-2 rounded-lg text-xs font-medium animate-pulse ${
            lastResult.type === "success"
              ? "bg-[#26E8A6]/20 text-[#26E8A6] border border-[#26E8A6]/30"
              : lastResult.type === "partial"
                ? "bg-yellow-500/20 text-yellow-400 border border-yellow-500/30"
                : "bg-red-500/20 text-red-400 border border-red-500/30"
          }`}
        >
          {lastResult.message}
          {lastResult.trades.length > 0 && (
            <div className="mt-1 text-[10px] opacity-80">
              {lastResult.trades.map((t, i) => (
                <div key={i}>
                  約定 #{i + 1}: {t.quantity} @ {t.price}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="flex justify-between text-xs text-zinc-400 mb-2">
        <span>Available to Trade</span>
        <span className="text-white font-mono">
          {parseFloat(availableBalance).toLocaleString()} {balanceAsset}
        </span>
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
              type="text"
              inputMode="decimal"
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
              placeholder="0"
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
          disabled={isSubmitting || !price || !quantity}
          className={`w-full py-3 rounded-lg font-bold text-lg mt-4 shadow-lg transition-all ${
            side === "Buy"
              ? "bg-[#26E8A6] hover:bg-[#20c990] text-black shadow-[#26E8A6]/20"
              : "bg-[#ff5353] hover:bg-[#e04848] text-white shadow-[#ff5353]/20"
          } ${isSubmitting ? "opacity-50 cursor-not-allowed" : ""} ${
            !price || !quantity ? "opacity-30 cursor-not-allowed" : ""
          }`}
        >
          {isSubmitting ? "送信中..." : `${side} BAD`}
        </button>
      </form>
    </div>
  );
}
