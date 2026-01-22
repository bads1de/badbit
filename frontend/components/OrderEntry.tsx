"use client";

import { useState } from "react";
import { Side } from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ArrowDownCircle, ArrowUpCircle } from "lucide-react";

export default function OrderEntry() {
  const [price, setPrice] = useState("");
  const [quantity, setQuantity] = useState("");
  const [side, setSide] = useState<Side>("Buy");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await fetch("http://localhost:8000/order", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          price: parseInt(price),
          quantity: parseInt(quantity),
          side,
        }),
      });
      setPrice("");
      setQuantity("");
    } catch (err) {
      console.error("Order failed", err);
    }
  };

  const total = price && quantity ? (parseInt(price) * parseFloat(quantity)).toFixed(2) : "0.00";

  return (
    <Card className="bg-zinc-950 border-zinc-800 shadow-2xl">
      <CardHeader className="pb-4">
        <CardTitle className="text-sm font-bold uppercase tracking-widest text-zinc-400">Trade BAD</CardTitle>
      </CardHeader>
      <CardContent className="space-y-6">
        <Tabs defaultValue="Buy" onValueChange={(v) => setSide(v as Side)} className="w-full">
          <TabsList className="grid w-full grid-cols-2 bg-zinc-900 p-1">
            <TabsTrigger 
              value="Buy" 
              className="data-[state=active]:bg-emerald-600 data-[state=active]:text-white font-bold"
            >
              <ArrowUpCircle className="w-4 h-4 mr-2" />
              BUY
            </TabsTrigger>
            <TabsTrigger 
              value="Sell" 
              className="data-[state=active]:bg-rose-600 data-[state=active]:text-white font-bold"
            >
              <ArrowDownCircle className="w-4 h-4 mr-2" />
              SELL
            </TabsTrigger>
          </TabsList>
        </Tabs>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <Label className="text-[10px] text-zinc-500 font-bold uppercase">Price</Label>
              <span className="text-[10px] text-zinc-600 font-bold">USDT</span>
            </div>
            <Input
              type="number"
              value={price}
              onChange={(e) => setPrice(e.target.value)}
              className="bg-zinc-900 border-zinc-800 h-11 text-zinc-100 font-mono text-lg focus-visible:ring-zinc-700"
              placeholder="0.00"
              required
            />
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <Label className="text-[10px] text-zinc-500 font-bold uppercase">Size</Label>
              <span className="text-[10px] text-zinc-600 font-bold">BAD</span>
            </div>
            <Input
              type="number"
              value={quantity}
              onChange={(e) => setQuantity(e.target.value)}
              className="bg-zinc-900 border-zinc-800 h-11 text-zinc-100 font-mono text-lg focus-visible:ring-zinc-700"
              placeholder="0.0"
              required
            />
          </div>

          <div className="pt-2">
            <div className="flex justify-between text-[10px] text-zinc-500 font-bold mb-4 px-1">
              <span>ESTIMATED TOTAL</span>
              <span className="text-zinc-300 font-mono">{total} USDT</span>
            </div>
            <Button
              type="submit"
              className={`w-full h-14 font-black text-lg shadow-lg transition-all active:scale-[0.98] ${
                side === "Buy" 
                ? "bg-emerald-600 hover:bg-emerald-500 text-white" 
                : "bg-rose-600 hover:bg-rose-500 text-white"
              }`}
            >
              {side === "Buy" ? "BUY BAD" : "SELL BAD"}
            </Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}