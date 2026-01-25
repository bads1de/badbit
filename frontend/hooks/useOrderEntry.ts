import { useState, useEffect, useCallback } from "react";
import { Side, Trade, BalanceResponse } from "@/types";

export const useOrderEntry = () => {
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

  // Balance fetching logic
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
    const interval = setInterval(fetchBalance, 1000); // Poll every 1 second
    return () => clearInterval(interval);
  }, []);

  // Order submission logic
  const placeOrder = useCallback(async () => {
    if (!price || !quantity) return;

    setIsSubmitting(true);
    setLastResult(null);

    try {
      const res = await fetch("http://localhost:8000/order", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          // Backend receives Decimal as string
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
          // No trades (Limit order added to book)
          setLastResult({
            type: "success",
            message: `📋 指値注文が板に追加されました (${side} ${requestedQty} @ ${price})`,
            trades: [],
          });
        } else if (totalFilled < requestedQty) {
          // Partial fill
          setLastResult({
            type: "partial",
            message: `⚡ 部分約定: ${totalFilled}/${requestedQty} 約定`,
            trades,
          });
        } else {
          // Full fill
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

        // Reset form
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
      // Clear result after 5 seconds
      setTimeout(() => setLastResult(null), 5000);
    }
  }, [price, quantity, side]);

  return {
    price,
    setPrice,
    quantity,
    setQuantity,
    side,
    setSide,
    percent,
    setPercent,
    isSubmitting,
    lastResult,
    balances,
    placeOrder,
  };
};
