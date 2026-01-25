import { useBalances } from "@/hooks/useBalances";

export default function AssetsDisplay() {
  const balances = useBalances();

  const assets = [
    {
      name: "USDC",
      available: parseFloat(balances.usdc_available),
      locked: parseFloat(balances.usdc_locked),
    },
    {
      name: "BAD",
      available: parseFloat(balances.bad_available),
      locked: parseFloat(balances.bad_locked),
    },
  ];

  return (
    <div className="flex-1 overflow-auto">
      <table className="w-full text-left text-xs">
        <thead className="text-zinc-500 font-medium sticky top-0 bg-[#13141b] z-10">
          <tr className="border-b border-white/5">
            <th className="px-4 py-2 font-normal">Asset</th>
            <th className="px-4 py-2 font-normal text-right">Available</th>
            <th className="px-4 py-2 font-normal text-right">In Orders</th>
            <th className="px-4 py-2 font-normal text-right">Total</th>
          </tr>
        </thead>
        <tbody>
          {assets.map((asset) => (
            <tr
              key={asset.name}
              className="hover:bg-white/5 transition-colors border-b border-white/5 last:border-0"
            >
              <td className="px-4 py-2 font-bold text-white">{asset.name}</td>
              <td className="px-4 py-2 text-right font-mono text-zinc-300">
                {asset.available.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 4,
                })}
              </td>
              <td className="px-4 py-2 text-right font-mono text-zinc-400">
                {asset.locked.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 4,
                })}
              </td>
              <td className="px-4 py-2 text-right font-mono text-white">
                {(asset.available + asset.locked).toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 4,
                })}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
