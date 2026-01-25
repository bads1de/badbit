import { useState, useEffect } from "react";
import { BalanceResponse } from "@/types";

export function useBalances() {
  const [balances, setBalances] = useState<BalanceResponse>({
    usdc_available: "0",
    usdc_locked: "0",
    bad_available: "0",
    bad_locked: "0",
  });

  useEffect(() => {
    const fetchBalances = async () => {
      try {
        const res = await fetch("http://localhost:8000/balance");
        if (res.ok) {
          const data = await res.json();
          setBalances(data);
        }
      } catch (err) {
        console.error("Balance fetch error:", err);
      }
    };

    fetchBalances(); // 初回実行
    const interval = setInterval(fetchBalances, 1000); // 1秒ごとに更新
    return () => clearInterval(interval);
  }, []);

  return balances;
}
