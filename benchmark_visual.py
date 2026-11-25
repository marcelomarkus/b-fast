#!/usr/bin/env python3
"""
Modern Disruptive Dark-Mode Benchmark Graphic for B-FAST
Generates high-resolution benchmark_chart.png with KPI hero cards and 4 comparison panels.
"""

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np


def generate_chart():
    # Setup modern typography and dark theme
    plt.rcParams.update(
        {
            "font.family": "sans-serif",
            "font.sans-serif": ["Inter", "DejaVu Sans", "Arial"],
            "text.color": "#F8FAFC",
            "axes.labelcolor": "#94A3B8",
            "xtick.color": "#94A3B8",
            "ytick.color": "#F8FAFC",
            "axes.edgecolor": "#1E293B",
            "axes.linewidth": 1.0,
            "grid.color": "#1E293B",
            "grid.linestyle": "--",
            "grid.alpha": 0.5,
        }
    )

    fig = plt.figure(figsize=(24, 14.5), facecolor="#0B0F19")

    # Header Title & Subtitle
    fig.text(
        0.05,
        0.955,
        "B-FAST BENCHMARK SUITE",
        fontsize=24,
        fontweight="bold",
        color="#F8FAFC",
        ha="left",
    )
    fig.text(
        0.05,
        0.932,
        "High-Performance Binary Wire Protocol for Python & TypeScript • Empirical Results vs Protobuf, orjson & JSON",
        fontsize=12,
        color="#64748B",
        ha="left",
    )

    # -------------------------------------------------------------------------
    # 3 Top KPI Highlight Cards
    # -------------------------------------------------------------------------
    kpis = [
        ("676 ns", "SUB-MICROSECOND LATENCY", "100 structured objects serialized in nanoseconds"),
        ("4.6x FASTER", "VS PROTOBUF DECODE", "Immediate native Python dicts vs slow MessageToDict"),
        ("3.18M FPS", "STREAMING THROUGHPUT", "Real-time sustained frames/sec (145x vs NDJSON)"),
    ]

    card_xs = [0.05, 0.36, 0.67]
    card_w = 0.28
    for (val, title, desc), x in zip(kpis, card_xs):
        # Card background
        rect = plt.Rectangle(
            (x, 0.825),
            card_w,
            0.085,
            transform=fig.transFigure,
            facecolor="#111827",
            edgecolor="#1E293B",
            linewidth=1.2,
            clip_on=False,
            zorder=1,
        )
        fig.patches.append(rect)
        # Accent left border
        accent = plt.Rectangle(
            (x, 0.825),
            0.004,
            0.085,
            transform=fig.transFigure,
            facecolor="#10B981",
            edgecolor="none",
            clip_on=False,
            zorder=2,
        )
        fig.patches.append(accent)

        fig.text(x + 0.015, 0.885, title, fontsize=9.5, fontweight="bold", color="#10B981", transform=fig.transFigure)
        fig.text(x + 0.015, 0.852, val, fontsize=17, fontweight="bold", color="#F8FAFC", transform=fig.transFigure)
        fig.text(x + 0.015, 0.835, desc, fontsize=8.5, color="#64748B", transform=fig.transFigure)

    # -------------------------------------------------------------------------
    # Grid of 4 Deep-Dive Panels
    # -------------------------------------------------------------------------
    gs = fig.add_gridspec(2, 2, top=0.79, bottom=0.06, left=0.05, right=0.95, hspace=0.30, wspace=0.18)

    c_bfast = "#10B981"      # Emerald Neon
    c_bfast_comp = "#059669" # Emerald Deep
    c_proto = "#6366F1"      # Indigo
    c_orjson = "#0284C7"     # Sky Blue
    c_json = "#475569"       # Muted Slate

    # -------------------------------------------------------------------------
    # Panel 1: B-FAST vs Protobuf (Python End-to-End)
    # -------------------------------------------------------------------------
    ax1 = fig.add_subplot(gs[0, 0])
    ax1.set_facecolor("#111827")
    labels1 = ["Encode\n(from Model)", "Full Decode\n(to Native Dicts)"]
    bfast_p1 = [7.21, 18.88]
    proto_p1 = [13.46, 87.13]
    x1 = np.arange(len(labels1))
    w1 = 0.32

    r1 = ax1.bar(x1 - w1/2, bfast_p1, w1, label="B-FAST", color=c_bfast, edgecolor="#065F46", linewidth=1.0, zorder=3)
    r2 = ax1.bar(x1 + w1/2, proto_p1, w1, label="Protobuf", color=c_proto, edgecolor="#4338CA", linewidth=1.0, zorder=3)

    ax1.set_xticks(x1)
    ax1.set_xticklabels(labels1, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax1.set_ylabel("Execution Time (ms) - Lower is better", fontsize=10.5, color="#94A3B8")
    ax1.set_title("1. B-FAST vs Protobuf — Real-World Python (10,000 Objects)", fontsize=13, fontweight="bold", color="#F8FAFC", pad=12, loc="left")
    ax1.set_ylim(0, 115)
    ax1.grid(True, axis="y", zorder=0)
    ax1.legend(loc="upper left", frameon=True, facecolor="#0B0F19", edgecolor="#1E293B", fontsize=10, labelcolor="#F8FAFC")

    # Values & badges
    ax1.text(r1[0].get_x() + r1[0].get_width()/2, 7.21 + 2.5, "7.2 ms", ha="center", fontsize=10, fontweight="bold", color="#F8FAFC")
    ax1.text(r2[0].get_x() + r2[0].get_width()/2, 13.46 + 2.5, "13.5 ms", ha="center", fontsize=10, fontweight="bold", color="#94A3B8")
    ax1.text(r1[0].get_x() + r1[0].get_width()/2, 28, "1.9x FASTER\nthan Protobuf", ha="center", fontsize=8.5, fontweight="bold", color="#10B981",
             bbox={"boxstyle": "round,pad=0.3", "facecolor": "#064E3B", "edgecolor": "#10B981", "alpha": 0.8})

    ax1.text(r1[1].get_x() + r1[1].get_width()/2, 18.88 + 2.5, "18.9 ms", ha="center", fontsize=10, fontweight="bold", color="#F8FAFC")
    ax1.text(r2[1].get_x() + r2[1].get_width()/2, 87.13 + 2.5, "87.1 ms", ha="center", fontsize=10, fontweight="bold", color="#94A3B8")
    ax1.text(r1[1].get_x() + r1[1].get_width()/2, 45, "4.6x FASTER\nthan Protobuf", ha="center", fontsize=8.5, fontweight="bold", color="#10B981",
             bbox={"boxstyle": "round,pad=0.3", "facecolor": "#064E3B", "edgecolor": "#10B981", "alpha": 0.8})

    # -------------------------------------------------------------------------
    # Panel 2: Simple Objects (10,000) — Horizontal Ranking
    # -------------------------------------------------------------------------
    ax2 = fig.add_subplot(gs[0, 1])
    ax2.set_facecolor("#111827")
    cats2 = ["JSON (stdlib)", "orjson", "B-FAST"]
    times2 = [12.00, 8.19, 2.01]
    colors2 = [c_json, c_orjson, c_bfast]

    y2 = np.arange(len(cats2))
    bars2 = ax2.barh(y2, times2, height=0.48, color=colors2, edgecolor="#1E293B", linewidth=1.0, zorder=3)
    ax2.set_yticks(y2)
    ax2.set_yticklabels(cats2, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax2.set_xlabel("Time (ms) - Lower is better", fontsize=10.5, color="#94A3B8")
    ax2.set_title("2. Simple Objects (10,000) — Encoding Speed", fontsize=13, fontweight="bold", color="#F8FAFC", pad=12, loc="left")
    ax2.set_xlim(0, 16)
    ax2.grid(True, axis="x", zorder=0)

    for i, (b, val) in enumerate(zip(bars2, times2)):
        w = b.get_width()
        txt = f"{val:.2f} ms"
        if i == 2:
            txt += "   4.1x faster than orjson"
            ax2.text(w + 0.35, b.get_y() + b.get_height()/2, txt, va="center", fontsize=10.5, fontweight="bold", color="#10B981")
        else:
            ax2.text(w + 0.35, b.get_y() + b.get_height()/2, txt, va="center", fontsize=10, color="#94A3B8")

    # -------------------------------------------------------------------------
    # Panel 3: Wire Payload Size (100,000 Objects)
    # -------------------------------------------------------------------------
    ax3 = fig.add_subplot(gs[1, 0])
    ax3.set_facecolor("#111827")
    cats3 = ["JSON", "orjson", "B-FAST", "B-FAST (Comp)"]
    sizes3 = [59.11, 55.29, 45.11, 5.75]
    colors3 = [c_json, c_orjson, c_bfast, c_bfast_comp]

    y3 = np.arange(len(cats3))
    bars3 = ax3.barh(y3, sizes3, height=0.52, color=colors3, edgecolor="#1E293B", linewidth=1.0, zorder=3)
    ax3.set_yticks(y3)
    ax3.set_yticklabels(cats3, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax3.set_xlabel("Payload Size (MB) - Lower is better", fontsize=10.5, color="#94A3B8")
    ax3.set_title("3. Wire Payload Size (100,000 Structured Objects)", fontsize=13, fontweight="bold", color="#F8FAFC", pad=12, loc="left")
    ax3.set_xlim(0, 75)
    ax3.grid(True, axis="x", zorder=0)

    for i, (b, val) in enumerate(zip(bars3, sizes3)):
        w = b.get_width()
        txt = f"{val:.2f} MB"
        if i == 3:
            txt += "   90% bandwidth savings"
            ax3.text(w + 1.2, b.get_y() + b.get_height()/2, txt, va="center", fontsize=10.5, fontweight="bold", color="#34D399")
        else:
            ax3.text(w + 1.2, b.get_y() + b.get_height()/2, txt, va="center", fontsize=10, color="#94A3B8")

    # -------------------------------------------------------------------------
    # Panel 4: Network Round-Trip 100k Objects (100 Mbps)
    # -------------------------------------------------------------------------
    ax4 = fig.add_subplot(gs[1, 1])
    ax4.set_facecolor("#111827")
    cats4 = ["JSON", "orjson", "B-FAST (Comp)"]
    times4 = [5478, 4898, 1457]
    colors4 = [c_json, c_orjson, c_bfast_comp]

    y4 = np.arange(len(cats4))
    bars4 = ax4.barh(y4, times4, height=0.48, color=colors4, edgecolor="#1E293B", linewidth=1.0, zorder=3)
    ax4.set_yticks(y4)
    ax4.set_yticklabels(cats4, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax4.set_xlabel("Total Time Encode + Wire + Decode (ms) - Lower is better", fontsize=10.5, color="#94A3B8")
    ax4.set_title("4. Round-Trip Time (100,000 Objects @ 100 Mbps Network)", fontsize=13, fontweight="bold", color="#F8FAFC", pad=12, loc="left")
    ax4.set_xlim(0, 7000)
    ax4.grid(True, axis="x", zorder=0)

    for i, (b, val) in enumerate(zip(bars4, times4)):
        w = b.get_width()
        txt = f"{val:,} ms"
        if i == 2:
            txt += "   3.4x faster round-trip"
            ax4.text(w + 120, b.get_y() + b.get_height()/2, txt, va="center", fontsize=10.5, fontweight="bold", color="#34D399")
        else:
            ax4.text(w + 120, b.get_y() + b.get_height()/2, txt, va="center", fontsize=10, color="#94A3B8")

    plt.savefig("benchmark_chart.png", dpi=180, facecolor=fig.get_facecolor(), edgecolor="none")
    print("✅ benchmark_chart.png successfully generated with modern dark-mode aesthetic!")


if __name__ == "__main__":
    generate_chart()
