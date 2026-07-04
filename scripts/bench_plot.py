#!/usr/bin/env python3
"""
HTTP 压力测试
需要: oha, matplotlib, pandas
"""

import os
import random
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path

import matplotlib.pyplot as plt
import pandas as pd

BASE_URL = os.environ.get("BASE_URL", "http://localhost:8080")
DURATION = os.environ.get("DURATION", "10s")
CONNECTIONS = int(os.environ.get("CONNECTIONS", "100"))
OUTPUT_DIR = Path(os.environ.get("OUTPUT_DIR", "bench_results"))


@dataclass
class Scenario:
    name: str
    duration: str
    connections: int
    method: str
    content_type: str
    body_mode: str  # hot | multi_host | multi_page | steady
    body_count: int
    url: str


def ensure_oha():
    if shutil.which("oha") is None:
        print("错误：未找到 oha，请先安装：cargo install oha")
        sys.exit(1)


def generate_bodies(path: Path, mode: str, count: int):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        for _ in range(count):
            if mode == "hot":
                url = "https://hot.example.com/hot-post"
            elif mode == "multi_host":
                host = f"site-{random.randint(0, 9999):05d}.example.com"
                url = f"https://{host}/page"
            elif mode == "multi_page":
                page = random.randint(1, 10000)
                url = f"https://single.example.com/post/{page}"
            elif mode == "steady":
                host = f"steady-{random.randint(0, 999):04d}.example.com"
                page = random.randint(1, 1000)
                url = f"https://{host}/page/{page}"
            else:
                url = "https://example.com/"
            f.write(f'{{"url":"{url}","is_new_uv":false}}\n')


def run_oha(
    name: str,
    duration: str,
    connections: int,
    url: str,
    method: str = "GET",
    content_type: str | None = None,
    body_file: Path | None = None,
    output_file: Path | None = None,
) -> pd.DataFrame:
    print(f"\n=== {name} ===")
    cmd = [
        "oha",
        "-z", duration,
        "-c", str(connections),
        "--no-tui",
        "--output-format", "csv",
        "-m", method,
        url,
    ]
    if content_type:
        cmd.extend(["-T", content_type])
    if body_file:
        cmd.extend(["-Z", str(body_file)])

    result = subprocess.run(cmd, capture_output=True, text=True, check=True)
    df = pd.read_csv(pd.io.common.StringIO(result.stdout))

    if output_file:
        output_file.parent.mkdir(parents=True, exist_ok=True)
        df.to_csv(output_file, index=False)
        print(f"CSV 已保存: {output_file}")

    summary = summarize(df)
    print_summary(summary)
    return df


def summarize(df: pd.DataFrame) -> dict:
    total = len(df)
    success = (df["status"] == 200).sum()
    durations = df["request-duration"] * 1000  # ms
    return {
        "total": total,
        "success": int(success),
        "success_rate": success / total * 100 if total else 0,
        "rps": total / df["request-start"].max() if total else 0,
        "avg_ms": durations.mean(),
        "p50_ms": durations.median(),
        "p95_ms": durations.quantile(0.95),
        "p99_ms": durations.quantile(0.99),
        "max_ms": durations.max(),
        "min_ms": durations.min(),
    }


def print_summary(s: dict):
    print(f"  Success rate: {s['success_rate']:.2f}%")
    print(f"  Requests/sec: {s['rps']:.2f}")
    print(f"  Avg: {s['avg_ms']:.3f} ms")
    print(f"  P50: {s['p50_ms']:.3f} ms")
    print(f"  P95: {s['p95_ms']:.3f} ms")
    print(f"  P99: {s['p99_ms']:.3f} ms")
    print(f"  Min/Max: {s['min_ms']:.3f} / {s['max_ms']:.3f} ms")


def plot_results(results: list[tuple[str, dict]], output_dir: Path):
    names = [r[0] for r in results]
    rps = [r[1]["rps"] for r in results]
    avg = [r[1]["avg_ms"] for r in results]
    p95 = [r[1]["p95_ms"] for r in results]
    p99 = [r[1]["p99_ms"] for r in results]

    fig, axes = plt.subplots(2, 1, figsize=(12, 10))

    ax1 = axes[0]
    bars = ax1.bar(names, rps, color="steelblue")
    ax1.set_ylabel("Requests / sec")
    ax1.set_title("Throughput by Scenario")
    for bar, val in zip(bars, rps):
        ax1.text(bar.get_x() + bar.get_width() / 2, bar.get_height(),
                 f"{val:.0f}", ha="center", va="bottom", fontsize=8)
    plt.setp(ax1.xaxis.get_majorticklabels(), rotation=30, ha="right")

    ax2 = axes[1]
    x = range(len(names))
    width = 0.25
    ax2.bar([i - width for i in x], avg, width, label="Avg", color="skyblue")
    ax2.bar(x, p95, width, label="P95", color="orange")
    ax2.bar([i + width for i in x], p99, width, label="P99", color="coral")
    ax2.set_ylabel("Latency (ms)")
    ax2.set_title("Latency by Scenario")
    ax2.set_xticks(x)
    ax2.set_xticklabels(names)
    ax2.legend()
    plt.setp(ax2.xaxis.get_majorticklabels(), rotation=30, ha="right")

    plt.tight_layout()
    chart_path = output_dir / "benchmark_summary.png"
    plt.savefig(chart_path, dpi=150)
    print(f"\n图表已保存: {chart_path}")


def plot_ramp(ramp_results: list[tuple[int, dict]], output_dir: Path):
    connections = [r[0] for r in ramp_results]
    rps = [r[1]["rps"] for r in ramp_results]
    p99 = [r[1]["p99_ms"] for r in ramp_results]

    fig, ax1 = plt.subplots(figsize=(10, 5))
    color = "steelblue"
    ax1.set_xlabel("Connections")
    ax1.set_ylabel("Requests / sec", color=color)
    ax1.plot(connections, rps, marker="o", color=color)
    ax1.tick_params(axis="y", labelcolor=color)

    ax2 = ax1.twinx()
    color = "coral"
    ax2.set_ylabel("P99 latency (ms)", color=color)
    ax2.plot(connections, p99, marker="s", color=color)
    ax2.tick_params(axis="y", labelcolor=color)

    plt.title("Ramp-up: Throughput vs P99 Latency")
    plt.tight_layout()
    chart_path = output_dir / "benchmark_ramp.png"
    plt.savefig(chart_path, dpi=150)
    print(f"图表已保存: {chart_path}")


def main():
    ensure_oha()
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    tmpdir = Path(tempfile.mkdtemp(prefix="busrzi_bench_"))
    body_file = tmpdir / "bodies.txt"

    scenarios = [
        Scenario("hot_page", DURATION, CONNECTIONS, "POST", "application/json", "hot", 1000, f"{BASE_URL}/api/collect"),
        Scenario("multi_host", DURATION, CONNECTIONS, "POST", "application/json", "multi_host", 10000, f"{BASE_URL}/api/collect"),
        Scenario("multi_page", DURATION, CONNECTIONS, "POST", "application/json", "multi_page", 10000, f"{BASE_URL}/api/collect"),
        Scenario("steady", "60s", CONNECTIONS, "POST", "application/json", "steady", 50000, f"{BASE_URL}/api/collect"),
    ]

    results: list[tuple[str, dict]] = []

    for sc in scenarios:
        generate_bodies(body_file, sc.body_mode, sc.body_count)
        df = run_oha(
            name=sc.name,
            duration=sc.duration,
            connections=sc.connections,
            url=sc.url,
            method=sc.method,
            content_type=sc.content_type,
            body_file=body_file,
            output_file=OUTPUT_DIR / f"{sc.name}.csv",
        )
        results.append((sc.name, summarize(df)))

    # 混合读写
    print("\n=== mixed_read_write ===")
    generate_bodies(body_file, "hot", 1000)
    js_proc = subprocess.Popen(
        ["oha", "-z", DURATION, "-c", str(CONNECTIONS // 2), "--no-tui", "--output-format", "csv", f"{BASE_URL}/js"],
        stdout=subprocess.PIPE, text=True,
    )
    api_proc = subprocess.Popen(
        ["oha", "-z", DURATION, "-c", str(CONNECTIONS // 2), "--no-tui", "--output-format", "csv",
         "-m", "POST", "-T", "application/json", "-Z", str(body_file), f"{BASE_URL}/api/collect"],
        stdout=subprocess.PIPE, text=True,
    )
    js_stdout, _ = js_proc.communicate()
    api_stdout, _ = api_proc.communicate()

    js_df = pd.read_csv(pd.io.common.StringIO(js_stdout))
    api_df = pd.read_csv(pd.io.common.StringIO(api_stdout))
    js_df.to_csv(OUTPUT_DIR / "mixed_js.csv", index=False)
    api_df.to_csv(OUTPUT_DIR / "mixed_api.csv", index=False)
    results.append(("mixed_js", summarize(js_df)))
    results.append(("mixed_api", summarize(api_df)))
    print("  /js:")
    print_summary(summarize(js_df))
    print("  /api/collect:")
    print_summary(summarize(api_df))

    # 并发梯度
    ramp_results: list[tuple[int, dict]] = []
    generate_bodies(body_file, "hot", 1000)
    for c in [10, 50, 100, 200, 500]:
        if c > CONNECTIONS:
            continue
        df = run_oha(
            name=f"ramp_{c}",
            duration=DURATION,
            connections=c,
            url=f"{BASE_URL}/api/collect",
            method="POST",
            content_type="application/json",
            body_file=body_file,
            output_file=OUTPUT_DIR / f"ramp_{c}.csv",
        )
        ramp_results.append((c, summarize(df)))
        results.append((f"ramp_{c}", summarize(df)))

    # 汇总 CSV
    summary_df = pd.DataFrame(
        [{"scenario": name, **stats} for name, stats in results]
    )
    summary_path = OUTPUT_DIR / "summary.csv"
    summary_df.to_csv(summary_path, index=False)
    print(f"\n汇总 CSV 已保存: {summary_path}")

    # 画图
    plot_results(results, OUTPUT_DIR)
    plot_ramp(ramp_results, OUTPUT_DIR)

    # 清理
    shutil.rmtree(tmpdir, ignore_errors=True)


if __name__ == "__main__":
    main()
