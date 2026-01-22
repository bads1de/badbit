"use client";

import { useEffect, useRef } from "react";
import {
  createChart,
  ColorType,
  ISeriesApi,
  CandlestickData,
  Time,
  CandlestickSeries,
  IChartApi,
} from "lightweight-charts";

interface Props {
  trades: { price: number; timestamp: number }[];
}

export default function PriceChart({ trades }: Props) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const seriesRef = useRef<ISeriesApi<"Candlestick">>(null);
  const chartRef = useRef<IChartApi | null>(null);

  useEffect(() => {
    if (!chartContainerRef.current) return;

    const chart = createChart(chartContainerRef.current, {
      layout: {
        background: { type: ColorType.Solid, color: "#13141b" },
        textColor: "#5e606e",
      },
      grid: {
        vertLines: { color: "#1e1f29" },
        horzLines: { color: "#1e1f29" },
      },
      width: chartContainerRef.current.clientWidth,
      height: chartContainerRef.current.clientHeight, // Use parent height
      timeScale: {
        borderColor: "#1e1f29",
        timeVisible: true,
      },
      rightPriceScale: {
        borderColor: "#1e1f29",
      },
    });

    const series = chart.addSeries(CandlestickSeries, {
      upColor: "#26E8A6",
      downColor: "#ff5353",
      borderVisible: false,
      wickUpColor: "#26E8A6",
      wickDownColor: "#ff5353",
    });

    seriesRef.current = series;
    chartRef.current = chart;

    const handleResize = () => {
      if (chartContainerRef.current) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
          height: chartContainerRef.current.clientHeight,
        });
      }
    };

    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      chart.remove();
    };
  }, []);

  useEffect(() => {
    if (!seriesRef.current || trades.length === 0) return;

    const candles: Record<number, CandlestickData<Time>> = {};
    const sortedTrades = [...trades].sort((a, b) => a.timestamp - b.timestamp);

    sortedTrades.forEach((t) => {
      const minute = Math.floor(t.timestamp / 60000) * 60;
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
    // Fit content optionally
    // chartRef.current?.timeScale().fitContent();
  }, [trades]);

  return <div ref={chartContainerRef} className="w-full h-full min-h-100" />;
}
