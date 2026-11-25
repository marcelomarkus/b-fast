#!/usr/bin/env python3
"""
Modern Disruptive Dark-Mode Benchmark Graphic for B-FAST.
Aligned with the official B-FAST brand identity:
- Electric Gold / Yellow (#FACC15) matching the signature lightning bolt
- Dark high-contrast slate backdrop (#0B0F19, #111827, #1E293B)
- Full 6-panel performance suite (Protobuf E2E, Encoding, Payload, Streaming, Round-Trip)
- Embedded official B-FAST Logo in header
"""

import os
import re

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from PIL import Image


def get_or_render_logo():
    """Renders a clean static transparent PNG from docs/assets/bfast-white.svg"""
    logo_svg_path = os.path.join(
        os.path.dirname(__file__), "docs/assets/bfast-white.svg"
    )
    out_png_path = "/tmp/bfast_logo_header.png"

    if not os.path.exists(logo_svg_path):
        return None

    try:
        with open(logo_svg_path, encoding="utf-8") as f:
            svg = f.read()

        # Create static SVG where text has full opacity and guides/animations are omitted
        svg_clean = re.sub(r"animation:[^;}]+;", "", svg)
        svg_clean = svg_clean.replace("opacity: 0;", "opacity: 1;")
        svg_clean = re.sub(
            r'<g id="drawGuides">.*?</g>', "", svg_clean, flags=re.DOTALL
        )
        svg_clean = re.sub(r'<rect class="shine".*?/>', "", svg_clean)

        tmp_svg_path = "/tmp/bfast_clean_static.svg"
        with open(tmp_svg_path, "w", encoding="utf-8") as f:
            f.write(svg_clean)

        ret = os.system(
            f'magick -background none -density 300 "{tmp_svg_path}" -resize 650x "{out_png_path}"'
        )
        if ret == 0 and os.path.exists(out_png_path):
            return out_png_path
    except Exception:
        pass

    return None


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

    fig = plt.figure(figsize=(26, 16), facecolor="#0B0F19")

    # Header with embedded official logo
    logo_png = get_or_render_logo()
    if logo_png and os.path.exists(logo_png):
        logo_img = Image.open(logo_png)
        logo_ax = fig.add_axes([0.05, 0.915, 0.155, 0.065])
        logo_ax.imshow(logo_img)
        logo_ax.axis("off")
        title_x = 0.22
    else:
        title_x = 0.05

    fig.text(
        title_x,
        0.955,
        "PERFORMANCE BENCHMARK SUITE",
        fontsize=22,
        fontweight="bold",
        color="#F8FAFC",
        ha="left",
    )
    fig.text(
        title_x,
        0.932,
        "Empirical Performance vs Google Protobuf, orjson & JSON across Serialization, Deserialization, Network & Streaming",
        fontsize=11.5,
        color="#94A3B8",
        ha="left",
    )

    # -------------------------------------------------------------------------
    # 3 Top KPI Highlight Cards (Brand Gold Accent)
    # -------------------------------------------------------------------------
    kpis = [
        (
            "676 ns",
            "SUB-MICROSECOND SERIALIZATION",
            "100 structured objects serialized in nanoseconds",
        ),
        (
            "4.6x FASTER",
            "VS PROTOBUF FULL PYTHON DECODE",
            "Immediate native dicts vs slow MessageToDict (18.9ms vs 87.1ms)",
        ),
        (
            "3.18M FPS",
            "STREAMING PROTOCOL THROUGHPUT",
            "Real-time sustained frames/sec (145x faster than NDJSON)",
        ),
    ]

    card_xs = [0.05, 0.36, 0.67]
    card_w = 0.28
    for (val, title, desc), x in zip(kpis, card_xs):
        rect = plt.Rectangle(
            (x, 0.835),
            card_w,
            0.068,
            transform=fig.transFigure,
            facecolor="#111827",
            edgecolor="#1E293B",
            linewidth=1.2,
            clip_on=False,
            zorder=1,
        )
        fig.patches.append(rect)

        # Electric Gold accent bar matching logo lightning bolt
        accent = plt.Rectangle(
            (x, 0.835),
            0.004,
            0.068,
            transform=fig.transFigure,
            facecolor="#FACC15",
            edgecolor="none",
            clip_on=False,
            zorder=2,
        )
        fig.patches.append(accent)

        fig.text(
            x + 0.015,
            0.884,
            title,
            fontsize=9.5,
            fontweight="bold",
            color="#FACC15",
            transform=fig.transFigure,
        )
        fig.text(
            x + 0.015,
            0.856,
            val,
            fontsize=17,
            fontweight="bold",
            color="#F8FAFC",
            transform=fig.transFigure,
        )
        fig.text(
            x + 0.015,
            0.841,
            desc,
            fontsize=8.5,
            color="#94A3B8",
            transform=fig.transFigure,
        )

    # -------------------------------------------------------------------------
    # 6 Deep-Dive Panels (2 rows x 3 cols)
    # -------------------------------------------------------------------------
    gs = fig.add_gridspec(
        2, 3, top=0.80, bottom=0.065, left=0.065, right=0.96, hspace=0.32, wspace=0.24
    )

    # Official Brand Palette
    c_bfast = "#FACC15"  # Electric Yellow / Gold (matches logo bolt)
    c_bfast_comp = "#F59E0B"  # Golden Amber (matches secondary gradient)
    c_proto = "#6366F1"  # Indigo (Protobuf)
    c_orjson = "#38BDF8"  # Sky Blue (orjson)
    c_json = "#64748B"  # Slate (JSON / NDJSON)

    badge_style = {
        "boxstyle": "round,pad=0.35",
        "facecolor": "#3B2702",
        "edgecolor": "#FACC15",
        "alpha": 0.9,
    }

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

    r1 = ax1.bar(
        x1 - w1 / 2,
        bfast_p1,
        w1,
        label="B-FAST",
        color=c_bfast,
        edgecolor="#CA8A04",
        linewidth=1.0,
        zorder=3,
    )
    r2 = ax1.bar(
        x1 + w1 / 2,
        proto_p1,
        w1,
        label="Protobuf",
        color=c_proto,
        edgecolor="#4338CA",
        linewidth=1.0,
        zorder=3,
    )

    ax1.set_xticks(x1)
    ax1.set_xticklabels(labels1, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax1.set_ylabel(
        "Execution Time (ms) - Lower is better", fontsize=10, color="#94A3B8"
    )
    ax1.set_title(
        "1. B-FAST vs Protobuf (10,000 Objects E2E)",
        fontsize=12.5,
        fontweight="bold",
        color="#F8FAFC",
        pad=12,
        loc="left",
    )
    ax1.set_ylim(0, 115)
    ax1.grid(True, axis="y", zorder=0)
    ax1.legend(
        loc="upper left",
        frameon=True,
        facecolor="#0B0F19",
        edgecolor="#1E293B",
        fontsize=9.5,
        labelcolor="#F8FAFC",
    )

    ax1.text(
        r1[0].get_x() + r1[0].get_width() / 2,
        7.21 + 2.5,
        "7.2 ms",
        ha="center",
        fontsize=9.5,
        fontweight="bold",
        color="#F8FAFC",
    )
    ax1.text(
        r2[0].get_x() + r2[0].get_width() / 2,
        13.46 + 2.5,
        "13.5 ms",
        ha="center",
        fontsize=9.5,
        fontweight="bold",
        color="#94A3B8",
    )
    ax1.text(
        r1[0].get_x() + r1[0].get_width() / 2,
        28,
        "1.9x FASTER\nthan Protobuf",
        ha="center",
        fontsize=8.5,
        fontweight="bold",
        color="#FDE047",
        bbox=badge_style,
    )

    ax1.text(
        r1[1].get_x() + r1[1].get_width() / 2,
        18.88 + 2.5,
        "18.9 ms",
        ha="center",
        fontsize=9.5,
        fontweight="bold",
        color="#F8FAFC",
    )
    ax1.text(
        r2[1].get_x() + r2[1].get_width() / 2,
        87.13 + 2.5,
        "87.1 ms",
        ha="center",
        fontsize=9.5,
        fontweight="bold",
        color="#94A3B8",
    )
    ax1.text(
        r1[1].get_x() + r1[1].get_width() / 2,
        45,
        "4.6x FASTER\nthan Protobuf",
        ha="center",
        fontsize=8.5,
        fontweight="bold",
        color="#FDE047",
        bbox=badge_style,
    )

    # -------------------------------------------------------------------------
    # Panel 2: Simple Objects (10,000) — Encoding Speed
    # -------------------------------------------------------------------------
    ax2 = fig.add_subplot(gs[0, 1])
    ax2.set_facecolor("#111827")
    cats2 = ["JSON (stdlib)", "orjson", "B-FAST"]
    times2 = [12.00, 8.19, 2.01]
    colors2 = [c_json, c_orjson, c_bfast]

    y2 = np.arange(len(cats2))
    bars2 = ax2.barh(
        y2,
        times2,
        height=0.48,
        color=colors2,
        edgecolor="#1E293B",
        linewidth=1.0,
        zorder=3,
    )
    ax2.set_yticks(y2)
    ax2.set_yticklabels(cats2, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax2.set_xlabel("Time (ms) - Lower is better", fontsize=10, color="#94A3B8")
    ax2.set_title(
        "2. Simple Objects (10,000) — Encoding Speed",
        fontsize=12.5,
        fontweight="bold",
        color="#F8FAFC",
        pad=12,
        loc="left",
    )
    ax2.set_xlim(0, 16)
    ax2.grid(True, axis="x", zorder=0)

    for i, (b, val) in enumerate(zip(bars2, times2)):
        w = b.get_width()
        txt = f"{val:.2f} ms"
        if i == 2:
            txt += "   4.1x faster than orjson"
            ax2.text(
                w + 0.35,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=10,
                fontweight="bold",
                color="#FACC15",
            )
        else:
            ax2.text(
                w + 0.35,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=9.5,
                color="#94A3B8",
            )

    # -------------------------------------------------------------------------
    # Panel 3: Complex Nested Objects (100,000) — Serialization Time
    # -------------------------------------------------------------------------
    ax3 = fig.add_subplot(gs[0, 2])
    ax3.set_facecolor("#111827")
    cats3 = ["JSON (stdlib)", "orjson", "B-FAST"]
    times3 = [170.0, 98.0, 1.46]
    colors3 = [c_json, c_orjson, c_bfast]

    y3 = np.arange(len(cats3))
    bars3 = ax3.barh(
        y3,
        times3,
        height=0.48,
        color=colors3,
        edgecolor="#1E293B",
        linewidth=1.0,
        zorder=3,
    )
    ax3.set_xscale("log")
    ax3.set_yticks(y3)
    ax3.set_yticklabels(cats3, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax3.set_xlabel(
        "Time in ms (Log Scale) - Lower is better", fontsize=10, color="#94A3B8"
    )
    ax3.set_title(
        "3. Complex Objects (100,000) — Serialization Time",
        fontsize=12.5,
        fontweight="bold",
        color="#F8FAFC",
        pad=12,
        loc="left",
    )
    ax3.set_xlim(0.8, 400)
    ax3.grid(True, axis="x", which="both", zorder=0)

    for i, (b, val) in enumerate(zip(bars3, times3)):
        w = b.get_width()
        txt = f"{val:.2f} ms"
        if i == 2:
            txt += "   67x faster than orjson"
            ax3.text(
                w * 1.25,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=10,
                fontweight="bold",
                color="#FACC15",
            )
        else:
            ax3.text(
                w * 1.15,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=9.5,
                color="#94A3B8",
            )

    # -------------------------------------------------------------------------
    # Panel 4: Wire Payload Size (100,000 Objects)
    # -------------------------------------------------------------------------
    ax4 = fig.add_subplot(gs[1, 0])
    ax4.set_facecolor("#111827")
    cats4 = ["JSON", "orjson", "B-FAST", "B-FAST (Comp)"]
    sizes4 = [59.11, 55.29, 45.11, 5.75]
    colors4 = [c_json, c_orjson, c_bfast, c_bfast_comp]

    y4 = np.arange(len(cats4))
    bars4 = ax4.barh(
        y4,
        sizes4,
        height=0.52,
        color=colors4,
        edgecolor="#1E293B",
        linewidth=1.0,
        zorder=3,
    )
    ax4.set_yticks(y4)
    ax4.set_yticklabels(cats4, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax4.set_xlabel("Payload Size (MB) - Lower is better", fontsize=10, color="#94A3B8")
    ax4.set_title(
        "4. Wire Payload Size (100,000 Objects)",
        fontsize=12.5,
        fontweight="bold",
        color="#F8FAFC",
        pad=12,
        loc="left",
    )
    ax4.set_xlim(0, 75)
    ax4.grid(True, axis="x", zorder=0)

    for i, (b, val) in enumerate(zip(bars4, sizes4)):
        w = b.get_width()
        txt = f"{val:.2f} MB"
        if i == 3:
            txt += "   90% smaller"
            ax4.text(
                w + 1.2,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=10,
                fontweight="bold",
                color="#FDE047",
            )
        else:
            ax4.text(
                w + 1.2,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=9.5,
                color="#94A3B8",
            )

    # -------------------------------------------------------------------------
    # Panel 5: Streaming Protocol (1,000 Frames) — Decode Time
    # -------------------------------------------------------------------------
    ax5 = fig.add_subplot(gs[1, 1])
    ax5.set_facecolor("#111827")
    cats5 = ["NDJSON (Line)", "B-FAST (Frag)", "B-FAST (Norm)"]
    times5 = [45.0, 0.32, 0.31]
    colors5 = [c_json, c_bfast_comp, c_bfast]

    y5 = np.arange(len(cats5))
    bars5 = ax5.barh(
        y5,
        times5,
        height=0.48,
        color=colors5,
        edgecolor="#1E293B",
        linewidth=1.0,
        zorder=3,
    )
    ax5.set_yticks(y5)
    ax5.set_yticklabels(cats5, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax5.set_xlabel("Time (ms) - Lower is better", fontsize=10, color="#94A3B8")
    ax5.set_title(
        "5. Streaming Protocol (1,000 Frames) — Decode Time",
        fontsize=12.5,
        fontweight="bold",
        color="#F8FAFC",
        pad=12,
        loc="left",
    )
    ax5.set_xlim(0, 58)
    ax5.grid(True, axis="x", zorder=0)

    for i, (b, val) in enumerate(zip(bars5, times5)):
        w = b.get_width()
        txt = f"{val:.2f} ms"
        if i == 2:
            txt += "   145x faster (314 µs)"
            ax5.text(
                w + 0.8,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=10,
                fontweight="bold",
                color="#FACC15",
            )
        elif i == 1:
            ax5.text(
                w + 0.8,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=9.5,
                color="#FDE047",
            )
        else:
            ax5.text(
                w + 0.8,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=9.5,
                color="#94A3B8",
            )

    # -------------------------------------------------------------------------
    # Panel 6: Network Round-Trip 100k Objects (100 Mbps)
    # -------------------------------------------------------------------------
    ax6 = fig.add_subplot(gs[1, 2])
    ax6.set_facecolor("#111827")
    cats6 = ["JSON", "orjson", "B-FAST (Comp)"]
    times6 = [5478, 4898, 1457]
    colors6 = [c_json, c_orjson, c_bfast_comp]

    y6 = np.arange(len(cats6))
    bars6 = ax6.barh(
        y6,
        times6,
        height=0.48,
        color=colors6,
        edgecolor="#1E293B",
        linewidth=1.0,
        zorder=3,
    )
    ax6.set_yticks(y6)
    ax6.set_yticklabels(cats6, fontsize=11, fontweight="bold", color="#F8FAFC")
    ax6.set_xlabel(
        "Round-Trip Time Encode+Wire+Decode (ms)", fontsize=10, color="#94A3B8"
    )
    ax6.set_title(
        "6. Network Round-Trip (100k Objects @ 100 Mbps)",
        fontsize=12.5,
        fontweight="bold",
        color="#F8FAFC",
        pad=12,
        loc="left",
    )
    ax6.set_xlim(0, 7200)
    ax6.grid(True, axis="x", zorder=0)

    for i, (b, val) in enumerate(zip(bars6, times6)):
        w = b.get_width()
        txt = f"{val:,} ms"
        if i == 2:
            txt += "   3.4x faster round-trip"
            ax6.text(
                w + 120,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=10,
                fontweight="bold",
                color="#FACC15",
            )
        else:
            ax6.text(
                w + 120,
                b.get_y() + b.get_height() / 2,
                txt,
                va="center",
                fontsize=9.5,
                color="#94A3B8",
            )

    # Save to benchmark_chart.png and doc asset directories
    out_file = "benchmark_chart.png"
    plt.savefig(out_file, dpi=180, facecolor=fig.get_facecolor(), edgecolor="none")
    import shutil

    for dest_dir in ["docs/assets", "docs/pt/docs/assets"]:
        if os.path.exists(dest_dir):
            shutil.copyfile(out_file, os.path.join(dest_dir, "benchmark_chart.png"))

    print("✅ benchmark_chart.png successfully generated with B-FAST brand colors!")


if __name__ == "__main__":
    generate_chart()
