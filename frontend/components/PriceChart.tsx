"use client";

import { useEffect, useRef } from "react";
import { createChart, ColorType, ISeriesApi, CandlestickData, Time, CandlestickSeries } from "lightweight-charts";

interface Props {
  trades: { price: number; timestamp: number }[];
}

export default function PriceChart({ trades }: Props) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const seriesRef = useRef<ISeriesApi<"Candlestick">>(null);

  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: "#0a0a0a" },
        textColor: "#a3a3a3",
      },
      grid: {
        vertLines: { color: "#171717" },
        horzLines: { color: "#171717" },
      },
      width: chartContainerRef.current.clientWidth,
      height: 300,
    });

    // In lightweight-charts v5, addCandlestickSeries is replaced by addSeries(CandlestickSeries, options)
    const series = chart.addSeries(CandlestickSeries, {
      upColor: "#10b981",
      downColor: "#e11d48",
      borderVisible: false,
      wickUpColor: "#10b981",
      wickDownColor: "#e11d48",
    });

    seriesRef.current = series;

    const handleResize = () => {
      chart.applyOptions({ width: chartContainerRef.current?.clientWidth });
    };

    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      chart.remove();
    };
  }, []);

  useEffect(() => {
    if (!seriesRef.current || trades.length === 0) return;

    // Convert trades to 1-minute candlesticks
    const candles: Record<number, CandlestickData<Time>> = {};
    
    // Sort trades by timestamp (oldest first)
    const sortedTrades = [...trades].sort((a, b) => a.timestamp - b.timestamp);

    sortedTrades.forEach((t) => {
      const minute = Math.floor(t.timestamp / 60000) * 60; // 1-min resolution in seconds
      if (!candles[minute]) {
        candles[minute] = {
          time: minute as Time,
          open: t.price,
          high: t.price,
          low: t.price,
          close: t.price,
        };
      } else {
        const c = candles[minute];
        c.high = Math.max(c.high, t.price);
        c.low = Math.min(c.low, t.price);
        c.close = t.price;
      }
    });

    seriesRef.current.setData(Object.values(candles));
  }, [trades]);

  return <div ref={chartContainerRef} className="w-full h-[300px]" />;
}