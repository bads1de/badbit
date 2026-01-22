"use client";

import { useEffect, useRef } from "react";
import {
  createChart,
  ColorType,
  ISeriesApi,
  CandlestickData,
  HistogramData,
  Time,
  CandlestickSeries,
  HistogramSeries,
  IChartApi,
} from "lightweight-charts";

interface Props {
  trades: { price: number; volume: number; timestamp: number }[];
}

export default function PriceChart({ trades }: Props) {
  const chartContainerRef = useRef<HTMLDivElement>(null);
  const candlestickSeriesRef = useRef<ISeriesApi<"Candlestick">>(null);
  const volumeSeriesRef = useRef<ISeriesApi<"Histogram">>(null);
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
      height: chartContainerRef.current.clientHeight,
      timeScale: {
        borderColor: "#1e1f29",
        timeVisible: true,
      },
      rightPriceScale: {
        borderColor: "#1e1f29",
      },
    });

    // Candlestick Series
    const candlestickSeries = chart.addSeries(CandlestickSeries, {
      upColor: "#26E8A6",
      downColor: "#ff5353",
      borderVisible: false,
      wickUpColor: "#26E8A6",
      wickDownColor: "#ff5353",
    });

    // Volume Series
    const volumeSeries = chart.addSeries(HistogramSeries, {
      color: "#26a69a",
      priceFormat: {
        type: "volume",
      },
      priceScaleId: "", // Overlay on main chart
    });

    // Adjust price scale for volume to sit at bottom
    chart.priceScale("").applyOptions({
      scaleMargins: {
        top: 0.8, // Highest volume bar occupies bottom 20%
        bottom: 0,
      },
    });

    candlestickSeriesRef.current = candlestickSeries;
    volumeSeriesRef.current = volumeSeries;
    chartRef.current = chart;

    const handleResize = () => {
      if (chartContainerRef.current && chartRef.current) {
        chartRef.current.applyOptions({
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
    if (
      !candlestickSeriesRef.current ||
      !volumeSeriesRef.current ||
      trades.length === 0
    )
      return;

    const candles: Record<number, CandlestickData<Time>> = {};
    const volumes: Record<number, HistogramData<Time>> = {};

    const sortedTrades = [...trades].sort((a, b) => a.timestamp - b.timestamp);

    sortedTrades.forEach((t) => {
      const minute = Math.floor(t.timestamp / 60000) * 60;

      // Aggregate Candles
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

      // Aggregate Volume
      if (!volumes[minute]) {
        volumes[minute] = {
          time: minute as Time,
          value: t.volume,
          color: "#26a69a", // Default color, will update based on price action
        };
      } else {
        volumes[minute].value += t.volume;
      }
    });

    // Update Volume Colors based on Candle content
    const candleData = Object.values(candles);
    const volumeData = Object.values(volumes).map((v) => {
      const candle = candles[v.time as number];
      // If candle close >= open, green volume, else red
      const isUp = candle.close >= candle.open;
      return {
        ...v,
        color: isUp ? "rgba(38, 232, 166, 0.5)" : "rgba(255, 83, 83, 0.5)",
      };
    });

    candlestickSeriesRef.current.setData(candleData);
    volumeSeriesRef.current.setData(volumeData);
  }, [trades]);

  return <div ref={chartContainerRef} className="w-full h-full min-h-100" />;
}
