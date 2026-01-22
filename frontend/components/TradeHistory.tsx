"use client";

import { Trade } from "@/types";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";

interface Props {
  trades: Trade[];
}

export default function TradeHistory({ trades }: Props) {
  return (
    <Card className="bg-zinc-950 border-zinc-800 shadow-2xl h-full flex flex-col">
      <CardHeader className="pb-4 flex flex-row items-center justify-between space-y-0">
        <CardTitle className="text-sm font-bold uppercase tracking-widest text-zinc-400">Trade History</CardTitle>
        <Badge variant="secondary" className="bg-zinc-900 text-zinc-400 text-[10px]">{trades.length} TRADES</Badge>
      </CardHeader>
      <CardContent className="flex-1 min-h-0 p-0">
        <Table>
          <TableHeader className="bg-zinc-900/50 sticky top-0 z-10 border-b border-zinc-800">
            <TableRow className="border-none hover:bg-transparent">
              <TableHead className="h-10 text-[10px] font-bold text-zinc-500 uppercase">Price</TableHead>
              <TableHead className="h-10 text-[10px] font-bold text-zinc-500 uppercase text-right">Size</TableHead>
              <TableHead className="h-10 text-[10px] font-bold text-zinc-500 uppercase text-right">Time</TableHead>
            </TableRow>
          </TableHeader>
        </Table>
        <ScrollArea className="h-[calc(100%-40px)]">
          <Table>
            <TableBody>
              {trades.map((trade, i) => {
                const date = new Date(Number(trade.timestamp));
                const timeStr = date.toLocaleTimeString([], { hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit' });
                const isBuy = trade.maker_id < trade.taker_id; // Simple logic for demo

                return (
                  <TableRow key={i} className="border-zinc-900/50 hover:bg-white/5 transition-colors">
                    <TableCell className={`py-2 text-sm font-mono ${isBuy ? "text-emerald-500" : "text-rose-500"}`}>
                      {trade.price.toFixed(2)}
                    </TableCell>
                    <TableCell className="py-2 text-sm text-zinc-300 font-mono text-right">
                      {trade.quantity.toLocaleString()}
                    </TableCell>
                    <TableCell className="py-2 text-[10px] text-zinc-600 font-mono text-right">
                      {timeStr}
                    </TableCell>
                  </TableRow>
                );
              })}
              {trades.length === 0 && (
                <TableRow>
                  <TableCell colSpan={3} className="text-center py-20 text-zinc-700 italic text-xs">
                    No recent trade activity
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}