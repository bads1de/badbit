import { useMyOrders } from "@/hooks/useMyOrders";
import { Trash2 } from "lucide-react";

export default function OpenOrders() {
  const { myOrders, cancelOrder } = useMyOrders();

  if (myOrders.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center text-zinc-600 text-xs">
        <div className="flex flex-col items-center gap-2">
          <span>No open orders</span>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-auto">
      <table className="w-full text-left text-xs">
        <thead className="text-zinc-500 font-medium sticky top-0 bg-[#13141b] z-10">
          <tr className="border-b border-white/5">
            <th className="px-4 py-2 font-normal">Side</th>
            <th className="px-4 py-2 font-normal text-right">Price</th>
            <th className="px-4 py-2 font-normal text-right">Qty</th>
            <th className="px-4 py-2 font-normal text-right">Value</th>
            <th className="px-4 py-2 font-normal text-center">Action</th>
          </tr>
        </thead>
        <tbody>
          {myOrders.map((order) => {
            const price = parseFloat(order.price);
            const value = price * order.quantity;
            const isBuy = order.side === "Buy";

            return (
              <tr
                key={order.id}
                className="hover:bg-white/5 transition-colors border-b border-white/5 last:border-0"
              >
                <td
                  className={`px-4 py-2 font-bold ${isBuy ? "text-[#26E8A6]" : "text-[#ff5353]"}`}
                >
                  {order.side.toUpperCase()}
                </td>
                <td className="px-4 py-2 text-right font-mono text-white">
                  {price.toFixed(2)}
                </td>
                <td className="px-4 py-2 text-right font-mono text-zinc-300">
                  {order.quantity}
                </td>
                <td className="px-4 py-2 text-right font-mono text-zinc-400">
                  {value.toFixed(2)}
                </td>
                <td className="px-4 py-2 text-center">
                  <button
                    onClick={() => cancelOrder(order.id)}
                    className="p-1 hover:bg-white/10 rounded text-zinc-500 hover:text-red-400 transition-colors"
                    title="Cancel Order"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
