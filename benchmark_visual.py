#!/usr/bin/env python3
"""
Generate benchmark visualization for B-FAST
Generates benchmark_chart.png with balanced 7-panel layout:
  - Row 1: Simple Objects, NumPy Arrays, Payload Size, Network Round-Trip
  - Row 2: Streaming Decode Time, Streaming Throughput, B-FAST vs Protobuf (wide)
"""

import matplotlib.pyplot as plt
import numpy as np


def generate_chart():
    """Generate benchmark_chart.png with 7 panels including Streaming and Protobuf"""
    plt.rcParams.update(
        {
            "font.family": "sans-serif",
            "font.sans-serif": ["DejaVu Sans", "Arial", "Helvetica"],
            "axes.edgecolor": "#D1D5DB",
            "axes.linewidth": 1.2,
            "grid.color": "#E5E7EB",
            "grid.linestyle": "--",
            "grid.alpha": 0.7,
        }
    )

    fig = plt.figure(figsize=(24, 13), facecolor="#FAFAFA")
    fig.suptitle(
        "B-FAST Performance, Streaming & Protobuf Benchmarks",
        fontsize=24,
        fontweight="bold",
        color="#111827",
        y=0.98,
    )

    gs = fig.add_gridspec(2, 4, hspace=0.32, wspace=0.26)

    # Color palette
    c_bfast_comp = "#047857"  # Deep Emerald
    c_bfast = "#10B981"  # Emerald
    c_bfast_alt = "#14B8A6"  # Teal
    c_proto = "#6366F1"  # Indigo
    c_orjson = "#3B82F6"  # Blue
    c_json = "#EF4444"  # Red

    # -------------------------------------------------------------------------
    # 1. Simple Objects (10,000) - Encoding Speed
    # -------------------------------------------------------------------------
    ax1 = fig.add_subplot(gs[0, 0])
    categories1 = ["B-FAST", "orjson", "JSON"]
    times1 = [2.01, 8.19, 12.00]
    colors1 = [c_bfast, c_orjson, c_json]

    bars1 = ax1.bar(
        categories1,
        times1,
        color=colors1,
        width=0.55,
        edgecolor="#1F2937",
        linewidth=1.2,
        zorder=3,
    )
    ax1.set_facecolor("#FFFFFF")
    ax1.grid(True, axis="y", zorder=0)
    ax1.set_ylabel(
        "Time (ms) - Lower is better", fontsize=11, fontweight="bold", color="#374151"
    )
    ax1.set_title(
        "Simple Objects (10,000) - Encoding Speed",
        fontsize=12,
        fontweight="bold",
        color="#111827",
        pad=10,
    )
    ax1.set_ylim(0, max(times1) * 1.35)

    for i, (bar, time_val) in enumerate(zip(bars1, times1)):
        height = bar.get_height()
        ax1.text(
            bar.get_x() + bar.get_width() / 2.0,
            height + 0.3,
            f"{time_val:.2f} ms",
            ha="center",
            va="bottom",
            fontsize=10,
            fontweight="bold",
            color="#1F2937",
        )
        if i == 0:
            speedup = times1[1] / time_val
            ax1.text(
                bar.get_x() + bar.get_width() / 2.0,
                height + 1.8,
                f"{speedup:.1f}x faster\nthan orjson",
                ha="center",
                va="bottom",
                fontsize=9.0,
                color="#065F46",
                fontweight="bold",
                bbox={
                    "boxstyle": "round,pad=0.3",
                    "facecolor": "#D1FAE5",
                    "edgecolor": "#10B981",
                    "alpha": 0.9,
                },
            )

    # -------------------------------------------------------------------------
    # 2. NumPy Array (8MB = 1M Float64) - Speed (Log Scale)
    # -------------------------------------------------------------------------
    ax2 = fig.add_subplot(gs[0, 1])
    categories2 = ["B-FAST", "orjson", "JSON"]
    times2 = [3.29, 46.34, 318.21]
    colors2 = [c_bfast, c_orjson, c_json]

    bars2 = ax2.bar(
        categories2,
        times2,
        color=colors2,
        width=0.55,
        edgecolor="#1F2937",
        linewidth=1.2,
        zorder=3,
    )
    ax2.set_facecolor("#FFFFFF")
    ax2.set_yscale("log")
    ax2.grid(True, axis="y", which="both", zorder=0)
    ax2.set_ylabel(
        "Time (ms, log scale) - Lower is better",
        fontsize=11,
        fontweight="bold",
        color="#374151",
    )
    ax2.set_title(
        "NumPy Array (8 MB) - Speed",
        fontsize=12,
        fontweight="bold",
        color="#111827",
        pad=10,
    )
    ax2.set_ylim(1, 1000)

    for i, (bar, time_val) in enumerate(zip(bars2, times2)):
        height = bar.get_height()
        ax2.text(
            bar.get_x() + bar.get_width() / 2.0,
            height * 1.15,
            f"{time_val:.2f} ms",
            ha="center",
            va="bottom",
            fontsize=10,
            fontweight="bold",
            color="#1F2937",
        )
        if i == 0:
            speedup = times2[1] / time_val
            ax2.text(
                bar.get_x() + bar.get_width() / 2.0,
                height * 3.5,
                f"{speedup:.0f}x faster\nthan orjson",
                ha="center",
                va="bottom",
                fontsize=9.0,
                color="#065F46",
                fontweight="bold",
                bbox={
                    "boxstyle": "round,pad=0.3",
                    "facecolor": "#D1FAE5",
                    "edgecolor": "#10B981",
                    "alpha": 0.9,
                },
            )

    # -------------------------------------------------------------------------
    # 3. Payload Size / Bandwidth (100,000 Objects)
    # -------------------------------------------------------------------------
    ax3 = fig.add_subplot(gs[0, 2])
    categories3 = ["B-FAST (Comp)", "B-FAST", "orjson", "JSON"]
    sizes3 = [5.75, 45.11, 55.29, 59.11]
    colors3 = [c_bfast_comp, c_bfast, c_orjson, c_json]

    bars3 = ax3.bar(
        categories3,
        sizes3,
        color=colors3,
        width=0.6,
        edgecolor="#1F2937",
        linewidth=1.2,
        zorder=3,
    )
    ax3.set_facecolor("#FFFFFF")
    ax3.grid(True, axis="y", zorder=0)
    ax3.set_ylabel(
        "Payload Size (MB) - Lower is better",
        fontsize=11,
        fontweight="bold",
        color="#374151",
    )
    ax3.set_title(
        "Payload Size (100,000 Large Objects)",
        fontsize=12,
        fontweight="bold",
        color="#111827",
        pad=10,
    )
    ax3.set_ylim(0, max(sizes3) * 1.35)

    for i, (bar, size_val) in enumerate(zip(bars3, sizes3)):
        height = bar.get_height()
        ax3.text(
            bar.get_x() + bar.get_width() / 2.0,
            height + 1.2,
            f"{size_val:.1f} MB",
            ha="center",
            va="bottom",
            fontsize=10,
            fontweight="bold",
            color="#1F2937",
        )
        if i == 0:
            savings = (1 - size_val / sizes3[2]) * 100
            ax3.text(
                bar.get_x() + bar.get_width() / 2.0,
                height + 8.5,
                f"{savings:.0f}% smaller\nthan orjson",
                ha="center",
                va="bottom",
                fontsize=9.0,
                color="#065F46",
                fontweight="bold",
                bbox={
                    "boxstyle": "round,pad=0.3",
                    "facecolor": "#D1FAE5",
                    "edgecolor": "#047857",
                    "alpha": 0.9,
                },
            )

    # -------------------------------------------------------------------------
    # 4. Large Objects (100k) on 100 Mbps Network (Full Round-Trip)
    # -------------------------------------------------------------------------
    ax4 = fig.add_subplot(gs[0, 3])
    categories4 = ["B-FAST (Comp)", "orjson", "JSON"]
    times4 = [1457, 4898, 5478]
    colors4 = [c_bfast_comp, c_orjson, c_json]

    bars4 = ax4.bar(
        categories4,
        times4,
        color=colors4,
        width=0.55,
        edgecolor="#1F2937",
        linewidth=1.2,
        zorder=3,
    )
    ax4.set_facecolor("#FFFFFF")
    ax4.grid(True, axis="y", zorder=0)
    ax4.set_ylabel(
        "Total Time (ms) - Lower is better",
        fontsize=11,
        fontweight="bold",
        color="#374151",
    )
    ax4.set_title(
        "Round-Trip 100k Objects (100 Mbps)",
        fontsize=12,
        fontweight="bold",
        color="#111827",
        pad=10,
    )
    ax4.set_ylim(0, max(times4) * 1.35)

    for i, (bar, time_val) in enumerate(zip(bars4, times4)):
        height = bar.get_height()
        ax4.text(
            bar.get_x() + bar.get_width() / 2.0,
            height + 80,
            f"{time_val:.0f} ms",
            ha="center",
            va="bottom",
            fontsize=10,
            fontweight="bold",
            color="#1F2937",
        )
        if i == 0:
            speedup = times4[1] / time_val
            ax4.text(
                bar.get_x() + bar.get_width() / 2.0,
                height + 700,
                f"{speedup:.1f}x faster\nthan orjson",
                ha="center",
                va="bottom",
                fontsize=9.0,
                color="#065F46",
                fontweight="bold",
                bbox={
                    "boxstyle": "round,pad=0.3",
                    "facecolor": "#D1FAE5",
                    "edgecolor": "#047857",
                    "alpha": 0.9,
                },
            )

    # -------------------------------------------------------------------------
    # 5. Streaming Protocol (1,000 Frames) - Decode Time
    # -------------------------------------------------------------------------
    ax5 = fig.add_subplot(gs[1, 0])
    categories5 = ["B-FAST (Norm)", "B-FAST (Frag)", "NDJSON (Line)"]
    times5 = [0.31, 0.32, 45.0]
    colors5 = [c_bfast, c_bfast_alt, c_json]

    bars5 = ax5.bar(
        categories5,
        times5,
        color=colors5,
        width=0.55,
        edgecolor="#1F2937",
        linewidth=1.2,
        zorder=3,
    )
    ax5.set_facecolor("#FFFFFF")
    ax5.grid(True, axis="y", zorder=0)
    ax5.set_ylabel(
        "Time (ms) - Lower is better", fontsize=11, fontweight="bold", color="#374151"
    )
    ax5.set_title(
        "Streaming Protocol (1,000 Frames) - Decode Time",
        fontsize=12,
        fontweight="bold",
        color="#111827",
        pad=10,
    )
    ax5.set_ylim(0, max(times5) * 1.35)

    for i, (bar, time_val) in enumerate(zip(bars5, times5)):
        height = bar.get_height()
        display_str = f"{time_val:.2f} ms" if time_val < 1.0 else f"{time_val:.1f} ms"
        ax5.text(
            bar.get_x() + bar.get_width() / 2.0,
            height + 0.8,
            display_str,
            ha="center",
            va="bottom",
            fontsize=10,
            fontweight="bold",
            color="#1F2937",
        )
        if i == 0:
            speedup = times5[2] / time_val
            ax5.text(
                bar.get_x() + bar.get_width() / 2.0,
                height + 7.5,
                f"{speedup:.0f}x faster\n(314 µs total)",
                ha="center",
                va="bottom",
                fontsize=9.0,
                color="#065F46",
                fontweight="bold",
                bbox={
                    "boxstyle": "round,pad=0.3",
                    "facecolor": "#D1FAE5",
                    "edgecolor": "#10B981",
                    "alpha": 0.9,
                },
            )

    # -------------------------------------------------------------------------
    # 6. Streaming Throughput (Frames / Second)
    # -------------------------------------------------------------------------
    ax6 = fig.add_subplot(gs[1, 1])
    categories6 = ["B-FAST (Norm)", "B-FAST (Frag)", "NDJSON (Line)"]
    throughput6 = [3181000, 3099000, 22222]
    colors6 = [c_bfast, c_bfast_alt, c_json]

    bars6 = ax6.bar(
        categories6,
        throughput6,
        color=colors6,
        width=0.55,
        edgecolor="#1F2937",
        linewidth=1.2,
        zorder=3,
    )
    ax6.set_facecolor("#FFFFFF")
    ax6.grid(True, axis="y", zorder=0)
    ax6.set_ylabel(
        "Frames / Second - Higher is better",
        fontsize=11,
        fontweight="bold",
        color="#374151",
    )
    ax6.set_title(
        "Streaming Throughput (Frames / Sec)",
        fontsize=12,
        fontweight="bold",
        color="#111827",
        pad=10,
    )
    ax6.set_ylim(0, max(throughput6) * 1.35)
    ax6.yaxis.set_major_formatter(
        plt.FuncFormatter(lambda x, p: f"{x*1e-6:.1f}M" if x >= 1e6 else f"{int(x):,}")
    )

    for i, (bar, tp_val) in enumerate(zip(bars6, throughput6)):
        height = bar.get_height()
        label = f"{tp_val/1e6:.2f}M fps" if tp_val >= 1e6 else f"{tp_val:,} fps"
        ax6.text(
            bar.get_x() + bar.get_width() / 2.0,
            height + 50000,
            label,
            ha="center",
            va="bottom",
            fontsize=10,
            fontweight="bold",
            color="#1F2937",
        )
        if i == 0:
            multiplier = tp_val / throughput6[2]
            ax6.text(
                bar.get_x() + bar.get_width() / 2.0,
                height + 450000,
                f"3.18M fps\n{multiplier:.0f}x throughput",
                ha="center",
                va="bottom",
                fontsize=9.0,
                color="#065F46",
                fontweight="bold",
                bbox={
                    "boxstyle": "round,pad=0.3",
                    "facecolor": "#D1FAE5",
                    "edgecolor": "#10B981",
                    "alpha": 0.9,
                },
            )

    # -------------------------------------------------------------------------
    # 7. B-FAST vs Protobuf: Real-World Python (10,000 Objects) [Wide Panel]
    # -------------------------------------------------------------------------
    ax7 = fig.add_subplot(gs[1, 2:])
    categories7 = ["Encode (from Model)", "Decode (to Native Dicts)"]
    bfast_times7 = [7.21, 18.88]
    proto_times7 = [13.46, 87.13]

    x7 = np.arange(len(categories7))
    w7 = 0.28

    rects7_1 = ax7.bar(
        x7 - w7 / 2,
        bfast_times7,
        w7,
        label="B-FAST",
        color=c_bfast,
        edgecolor="#1F2937",
        linewidth=1.2,
        zorder=3,
    )
    rects7_2 = ax7.bar(
        x7 + w7 / 2,
        proto_times7,
        w7,
        label="Protobuf",
        color=c_proto,
        edgecolor="#1F2937",
        linewidth=1.2,
        zorder=3,
    )

    ax7.set_facecolor("#FFFFFF")
    ax7.grid(True, axis="y", zorder=0)
    ax7.set_ylabel(
        "Time (ms) - Lower is better", fontsize=11, fontweight="bold", color="#374151"
    )
    ax7.set_title(
        "B-FAST vs Protobuf (10,000 Objects in Python)",
        fontsize=12,
        fontweight="bold",
        color="#111827",
        pad=10,
    )
    ax7.set_xticks(x7)
    ax7.set_xticklabels(categories7, fontsize=10.5, fontweight="bold", color="#1F2937")
    ax7.set_ylim(0, max(proto_times7) * 1.35)
    ax7.legend(loc="upper left", frameon=True, facecolor="#FFFFFF", edgecolor="#D1D5DB", fontsize=10.5)

    # Encode: B-FAST
    ax7.text(
        rects7_1[0].get_x() + rects7_1[0].get_width() / 2,
        rects7_1[0].get_height() + 1.2,
        f"{bfast_times7[0]:.1f} ms",
        ha="center",
        va="bottom",
        fontsize=10,
        fontweight="bold",
        color="#1F2937",
    )
    speedup_enc = proto_times7[0] / bfast_times7[0]
    ax7.text(
        rects7_1[0].get_x() + rects7_1[0].get_width() / 2,
        rects7_1[0].get_height() + 10.0,
        f"{speedup_enc:.1f}x faster\nthan Protobuf",
        ha="center",
        va="bottom",
        fontsize=9.0,
        color="#065F46",
        fontweight="bold",
        bbox={
            "boxstyle": "round,pad=0.3",
            "facecolor": "#D1FAE5",
            "edgecolor": "#10B981",
            "alpha": 0.9,
        },
    )

    # Encode: Protobuf
    ax7.text(
        rects7_2[0].get_x() + rects7_2[0].get_width() / 2,
        rects7_2[0].get_height() + 1.2,
        f"{proto_times7[0]:.1f} ms",
        ha="center",
        va="bottom",
        fontsize=10,
        fontweight="bold",
        color="#1F2937",
    )

    # Decode: B-FAST
    ax7.text(
        rects7_1[1].get_x() + rects7_1[1].get_width() / 2,
        rects7_1[1].get_height() + 1.2,
        f"{bfast_times7[1]:.1f} ms",
        ha="center",
        va="bottom",
        fontsize=10,
        fontweight="bold",
        color="#1F2937",
    )
    speedup_dec = proto_times7[1] / bfast_times7[1]
    ax7.text(
        rects7_1[1].get_x() + rects7_1[1].get_width() / 2,
        rects7_1[1].get_height() + 10.0,
        f"{speedup_dec:.1f}x faster\nthan Protobuf",
        ha="center",
        va="bottom",
        fontsize=9.0,
        color="#065F46",
        fontweight="bold",
        bbox={
            "boxstyle": "round,pad=0.3",
            "facecolor": "#D1FAE5",
            "edgecolor": "#10B981",
            "alpha": 0.9,
        },
    )

    # Decode: Protobuf
    ax7.text(
        rects7_2[1].get_x() + rects7_2[1].get_width() / 2,
        rects7_2[1].get_height() + 1.2,
        f"{proto_times7[1]:.1f} ms",
        ha="center",
        va="bottom",
        fontsize=10,
        fontweight="bold",
        color="#1F2937",
    )

    fig.subplots_adjust(top=0.92, bottom=0.06, left=0.04, right=0.98, hspace=0.32, wspace=0.22)
    plt.savefig("benchmark_chart.png", dpi=180, bbox_inches="tight")
    print(
        "✅ benchmark_chart.png successfully generated with 7 panels including Streaming and Protobuf!"
    )


if __name__ == "__main__":
    generate_chart()
