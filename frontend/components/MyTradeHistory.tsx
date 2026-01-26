import { useMyTrades } from "@/hooks/useMyTrades";

export default function MyTradeHistory() {
  const { myTrades } = useMyTrades();

  if (myTrades.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-600 text-xs">
        <div className="flex flex-col items-center gap-2">
          <span>No trade history</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto">
      <table className="w-full text-left text-xs">
        <thead className="text-zinc-500 font-medium sticky top-0 bg-[#13141b] z-10">
          <tr className="border-b border-white/5">
            <th className="px-4 py-2 font-normal text-right">Price</th>
            <th className="px-4 py-2 font-normal text-right">Qty</th>
            <th className="px-4 py-2 font-normal text-right">Value</th>
            <th className="px-4 py-2 font-normal text-right">Time</th>
          </tr>
        </thead>
        <tbody>
          {myTrades.map((trade, i) => {
            const price = parseFloat(trade.price);
            const value = price * trade.quantity;
            const time = new Date(trade.timestamp).toLocaleTimeString();

            // MakerかTakerか、あるいはBuyかSellかを判定する情報がTrade構造体に足りていない
            // 現状は簡易表示のみ

            return (
              <tr
                key={i} // trade.idがないのでindex (本来はidが必要)
                className="hover:bg-white/5 transition-colors border-b border-white/5 last:border-0"
              >
                <td className="px-4 py-2 text-right font-mono text-white">
                  {price.toFixed(2)}
                </td>
                <td className="px-4 py-2 text-right font-mono text-zinc-300">
                  {trade.quantity}
                </td>
                <td className="px-4 py-2 text-right font-mono text-zinc-400">
                  {value.toFixed(2)}
                </td>
                <td className="px-4 py-2 text-right font-mono text-zinc-500">
                  {time}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
