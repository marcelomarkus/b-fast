#!/usr/bin/env python3
"""
Generate benchmark visualization for B-FAST
Generates benchmark_chart.png with hybrid mode results
"""

import matplotlib.pyplot as plt


def generate_chart():
    """Generate benchmark_chart.png (hybrid mode results)"""
    plt.style.use("seaborn-v0_8-darkgrid")
    fig, ((ax1, ax2), (ax3, ax4)) = plt.subplots(2, 2, figsize=(16, 12))
    fig.suptitle("B-FAST Performance Benchmarks", fontsize=20, fontweight="bold")

    # 1. Simple Objects (10k) - Speed
    categories = ["B-FAST", "orjson", "JSON"]
    times = [4.83, 8.19, 12.0]
    colors = ["#2ecc71", "#3498db", "#e74c3c"]

    bars1 = ax1.bar(
        categories, times, color=colors, alpha=0.8, edgecolor="black", linewidth=1.5
    )
    ax1.set_ylabel("Time (ms)", fontsize=12, fontweight="bold")
    ax1.set_title(
        "Simple Objects (10,000) - Encoding Speed", fontsize=14, fontweight="bold"
    )
    ax1.set_ylim(0, max(times) * 1.2)

    for i, (bar, time) in enumerate(zip(bars1, times)):
        height = bar.get_height()
        ax1.text(
            bar.get_x() + bar.get_width() / 2.0,
            height,
            f"{time:.2f}ms",
            ha="center",
            va="bottom",
            fontsize=11,
            fontweight="bold",
        )
        if i == 0:
            ax1.text(
                bar.get_x() + bar.get_width() / 2.0,
                height * 1.1,
                "1.7x faster\nthan orjson",
                ha="center",
                va="bottom",
                fontsize=10,
                color="green",
                fontweight="bold",
            )

    # 2. Large Objects (100k) on 100 Mbps Network
    categories = ["B-FAST+LZ4", "orjson", "JSON"]
    times = [1457, 4898, 5478]
    colors = ["#2ecc71", "#3498db", "#e74c3c"]

    bars2 = ax2.bar(
        categories, times, color=colors, alpha=0.8, edgecolor="black", linewidth=1.5
    )
    ax2.set_ylabel("Time (ms)", fontsize=12, fontweight="bold")
    ax2.set_title(
        "Large Objects (100k) - 100 Mbps Network", fontsize=14, fontweight="bold"
    )
    ax2.set_ylim(0, max(times) * 1.2)

    for i, (bar, time) in enumerate(zip(bars2, times)):
        height = bar.get_height()
        ax2.text(
            bar.get_x() + bar.get_width() / 2.0,
            height,
            f"{time:.0f}ms",
            ha="center",
            va="bottom",
            fontsize=11,
            fontweight="bold",
        )
        if i == 0:
            ax2.text(
                bar.get_x() + bar.get_width() / 2.0,
                height * 1.1,
                "3.4x faster\nthan orjson",
                ha="center",
                va="bottom",
                fontsize=10,
                color="green",
                fontweight="bold",
            )

    # 3. NumPy Array (8MB)
    categories = ["B-FAST", "orjson", "JSON"]
    times = [3.29, 46.34, 318.21]
    colors = ["#2ecc71", "#3498db", "#e74c3c"]

    bars3 = ax3.bar(
        categories, times, color=colors, alpha=0.8, edgecolor="black", linewidth=1.5
    )
    ax3.set_ylabel("Time (ms)", fontsize=12, fontweight="bold")
    ax3.set_title("NumPy Array (8MB) - Encoding Speed", fontsize=14, fontweight="bold")
    ax3.set_yscale("log")

    for i, (bar, time) in enumerate(zip(bars3, times)):
        height = bar.get_height()
        ax3.text(
            bar.get_x() + bar.get_width() / 2.0,
            height,
            f"{time:.1f}ms",
            ha="center",
            va="bottom",
            fontsize=11,
            fontweight="bold",
        )
        if i == 0:
            ax3.text(
                bar.get_x() + bar.get_width() / 2.0,
                height * 2,
                "14x faster\nthan orjson",
                ha="center",
                va="bottom",
                fontsize=10,
                color="green",
                fontweight="bold",
            )

    # 4. Payload Size
    categories = ["B-FAST+LZ4", "B-FAST", "orjson", "JSON"]
    sizes = [5.75, 45.11, 55.29, 59.11]
    colors = ["#27ae60", "#2ecc71", "#3498db", "#e74c3c"]

    bars4 = ax4.bar(
        categories, sizes, color=colors, alpha=0.8, edgecolor="black", linewidth=1.5
    )
    ax4.set_ylabel("Size (MB)", fontsize=12, fontweight="bold")
    ax4.set_title("Payload Size (100k Large Objects)", fontsize=14, fontweight="bold")
    ax4.set_ylim(0, max(sizes) * 1.2)

    for i, (bar, size) in enumerate(zip(bars4, sizes)):
        height = bar.get_height()
        ax4.text(
            bar.get_x() + bar.get_width() / 2.0,
            height,
            f"{size:.1f}MB",
            ha="center",
            va="bottom",
            fontsize=11,
            fontweight="bold",
        )
        if i == 0:
            ax4.text(
                bar.get_x() + bar.get_width() / 2.0,
                height * 1.1,
                "90% smaller\nthan orjson",
                ha="center",
                va="bottom",
                fontsize=10,
                color="green",
                fontweight="bold",
            )

    plt.tight_layout()
    plt.savefig("benchmark_chart.png", dpi=150, bbox_inches="tight")
    print("✅ benchmark_chart.png generated")


if __name__ == "__main__":
    generate_chart()
