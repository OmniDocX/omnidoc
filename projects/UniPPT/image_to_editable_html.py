#!/usr/bin/env python3
"""图片转可编辑 HTML —— 复刻 UniPPT「图转可编辑 PPT」完整管线的独立脚本。

流程（与 UniPPT 浏览器端 ai_attachment_runtime.js / ai_proxy.rs 对齐）：
  1. OCR      DashScope qwen3.5-ocr advanced_recognition，取逐行文字 + 四点绝对坐标
  2. 掩码     复刻 buildTextMaskFromPixels：按背景色中值 + 颜色距离分位阈值选中文字像素
  3. 擦字     依次尝试 clean_image 直连(Big LaMa) → UniPPT 服务器代理 → 本地回退填充
  4. 生成     独立可编辑 HTML：擦字背景 + 每个 OCR 行一个 contenteditable 文本框，
              支持移动、增删、缩放、切换背景、再导出 HTML/JSON

配置来源（优先级从高到低）：命令行参数 > 环境变量 > .env.local / .env.clean-image.local
  OCR:  UNIPPT_OCR_KEY / UNIPPT_AI_KEY，UNIPPT_OCR_BASE / UNIPPT_AI_BASE，UNIPPT_OCR_MODEL
  擦字: UNIPPT_CLEAN_IMAGE_URL + UNIPPT_CLEAN_IMAGE_KEY（直连），或 --server 走 UniPPT 代理

依赖: pip install pillow requests numpy   （可选 scipy / opencv-python 提升本地回退质量）

用法:
  python image_to_editable_html.py poster.png                 # 输出到 poster-editable/
  python image_to_editable_html.py poster.png -o out --inpaint local
  python image_to_editable_html.py --self-test                # 离线自检，不访问网络
"""

from __future__ import annotations

import argparse
import base64
import html as html_module
import io
import json
import math
import os
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import numpy as np
import requests
from PIL import Image

DEFAULT_OCR_BASE = "https://dashscope.aliyuncs.com"
DEFAULT_OCR_MODEL = "qwen3.5-ocr"
DEFAULT_UNIPPT_SERVER = "http://127.0.0.1:8155"
DATA_URL_LIMIT = 18 * 1024 * 1024  # 与服务端 /api/ai/ocr 的 18 MiB 上限一致

MIME_BY_SUFFIX = {
    ".png": "image/png", ".jpg": "image/jpeg", ".jpeg": "image/jpeg",
    ".webp": "image/webp", ".gif": "image/gif", ".bmp": "image/bmp",
}


# ---------------------------------------------------------------------------
# 配置与通用工具
# ---------------------------------------------------------------------------

def load_env_file(path: Path) -> Dict[str, str]:
    """极简 dotenv：KEY=VALUE，忽略注释；值允许 JSON 风格双引号。"""
    values: Dict[str, str] = {}
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return values
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        value = value.strip()
        if len(value) >= 2 and value[0] == value[-1] and value[0] in "\"'":
            value = value[1:-1]
        values[key.strip()] = value
    return values


def find_env_values(filename: str) -> Dict[str, str]:
    """先找当前目录，再找脚本所在目录。"""
    for base in (Path.cwd(), Path(__file__).resolve().parent):
        candidate = base / filename
        if candidate.is_file():
            return load_env_file(candidate)
    return {}


def resolve_setting(cli_value: Optional[str], names: List[str],
                    file_values: Dict[str, str], default: str = "") -> str:
    if cli_value and cli_value.strip():
        return cli_value.strip()
    for name in names:
        value = os.environ.get(name, "").strip()
        if value:
            return value
    for name in names:
        value = file_values.get(name, "").strip()
        if value:
            return value
    return default


def clamp(value: float, low: float, high: float) -> float:
    return max(low, min(high, value))


def rgb_hex(color: Any) -> str:
    parts = [int(clamp(round(float(channel)), 0, 255)) for channel in list(color)[:3]]
    return "#" + "".join(f"{part:02x}" for part in parts)


def encode_data_url(image_bytes: bytes, mime: str) -> str:
    return f"data:{mime};base64," + base64.b64encode(image_bytes).decode("ascii")


def decode_data_url(value: str) -> bytes:
    meta, _, payload = value.partition(",")
    if not meta.startswith("data:") or "base64" not in meta:
        raise ValueError("不是 base64 图片 Data URL")
    return base64.b64decode(payload)


def png_bytes(image: Image.Image) -> bytes:
    buffer = io.BytesIO()
    image.save(buffer, format="PNG")
    return buffer.getvalue()


# ---------------------------------------------------------------------------
# 第 1 步：OCR（DashScope 原生多模态接口，与 ai_proxy.rs::ocr_detect 一致）
# ---------------------------------------------------------------------------

def dashscope_ocr_url(base: str) -> str:
    trimmed = base.rstrip("/")
    lower = trimmed.lower()
    if "dashscope" not in lower and "aliyun" not in lower:
        raise RuntimeError("OCR 接口目前仅支持 DashScope；请把 --api-base 指向 dashscope.aliyuncs.com")
    for marker in ("/compatible-mode/v1", "/api/v1"):
        index = lower.find(marker)
        if index >= 0:
            return trimmed[:index] + "/api/v1/services/aigc/multimodal-generation/generation"
    return trimmed + "/api/v1/services/aigc/multimodal-generation/generation"


def run_ocr(image_data_url: str, width: int, height: int, attachment_name: str,
            api_key: str, api_base: str, model: str) -> Dict[str, Any]:
    if len(image_data_url) > DATA_URL_LIMIT:
        raise RuntimeError("图片 Data URL 超过 18 MiB 上限，请先缩小图片")
    payload = {
        "model": model,
        "input": {"messages": [{"role": "user", "content": [{
            "image": image_data_url,
            "min_pixels": 3072,
            "max_pixels": 8388608,
            "enable_rotate": False,
        }]}]},
        "parameters": {"ocr_options": {"task": "advanced_recognition"}},
    }
    response = requests.post(
        dashscope_ocr_url(api_base),
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
        json=payload,
        timeout=(12, 120),
    )
    if response.status_code != 200:
        raise RuntimeError(f"OCR 上游 HTTP {response.status_code}：{response.text[:800]}")
    data = response.json()
    choices = data.get("output", {}).get("choices") or [{}]
    content = choices[0].get("message", {}).get("content") or [{}]
    words_info = (content[0] or {}).get("ocr_result", {}).get("words_info") or []

    regions: List[Dict[str, Any]] = []
    for word in words_info:
        text = str(word.get("text", "")).strip()
        location = word.get("location")
        if not text or not isinstance(location, list) or len(location) != 8:
            continue
        try:
            location = [float(v) for v in location]
        except (TypeError, ValueError):
            continue
        if any(not math.isfinite(v) for v in location):
            continue
        rotate_rect = word.get("rotate_rect")
        if isinstance(rotate_rect, list) and len(rotate_rect) == 5:
            try:
                rotate_rect = [float(v) for v in rotate_rect]
            except (TypeError, ValueError):
                rotate_rect = None
        else:
            rotate_rect = None
        regions.append({"text": text, "location": location, "rotateRect": rotate_rect})

    if not regions:
        raise RuntimeError(f"OCR 未返回文字坐标（request_id={data.get('request_id', 'unknown')}）")
    return {
        "ok": True,
        "model": model,
        "attachmentName": attachment_name,
        "image": {"width": width, "height": height},
        "regionCount": len(regions),
        "regions": regions,
        "requestId": data.get("request_id"),
    }


# ---------------------------------------------------------------------------
# 第 2 步：文字掩码（复刻 web/ai_attachment_runtime.js::buildTextMaskFromPixels）
# ---------------------------------------------------------------------------

def region_polygon(region: Dict[str, Any], width: int, height: int) -> Optional[Dict[str, Any]]:
    location = region.get("location")
    if not isinstance(location, list) or len(location) != 8:
        return None
    try:
        loc = [float(v) for v in location]
    except (TypeError, ValueError):
        return None
    if any(not math.isfinite(v) for v in loc):
        return None
    points = [(clamp(loc[i], 0, width - 1), clamp(loc[i + 1], 0, height - 1)) for i in (0, 2, 4, 6)]
    top_length = max(1.0, math.hypot(points[1][0] - points[0][0], points[1][1] - points[0][1]))
    left_length = max(1.0, math.hypot(points[3][0] - points[0][0], points[3][1] - points[0][1]))
    return {"points": points, "shortEdge": max(1.0, min(top_length, left_length))}


def points_in_polygon(px: np.ndarray, py: np.ndarray, points: List[Tuple[float, float]]) -> np.ndarray:
    inside = np.zeros(px.shape, dtype=bool)
    count = len(points)
    for current in range(count):
        ax, ay = points[current]
        bx, by = points[current - 1]
        denominator = by - ay
        if abs(denominator) < 1e-12:
            denominator = 1e-12
    # 射线法：与 JS 版逐边异或
        crosses = ((ay > py) != (by > py)) & (px < (bx - ax) * (py - ay) / denominator + ax)
        inside ^= crosses
    return inside


def edge_distances(px: np.ndarray, py: np.ndarray, points: List[Tuple[float, float]]) -> np.ndarray:
    best: Optional[np.ndarray] = None
    count = len(points)
    for current in range(count):
        ax, ay = points[current]
        bx, by = points[(current + 1) % count]
        dx, dy = bx - ax, by - ay
        length_sq = dx * dx + dy * dy
        if length_sq <= 1e-12:
            distance = np.hypot(px - ax, py - ay)
        else:
            t = np.clip(((px - ax) * dx + (py - ay) * dy) / length_sq, 0.0, 1.0)
            distance = np.hypot(px - (ax + t * dx), py - (ay + t * dy))
        best = distance if best is None else np.minimum(best, distance)
    return best


def build_text_mask(rgba: np.ndarray, regions: List[Dict[str, Any]]) -> Dict[str, Any]:
    """返回 mask(H,W uint8, 255=待擦除)、每区前景色、掩码像素数。"""
    height, width = rgba.shape[:2]
    mask = np.zeros((height, width), dtype=np.uint8)
    foreground_colors: List[Optional[str]] = []
    masked_pixel_count = 0

    for region in regions:
        foreground_colors.append(None)
        geometry = region_polygon(region, width, height)
        if not geometry:
            continue
        xs = [p[0] for p in geometry["points"]]
        ys = [p[1] for p in geometry["points"]]
        raw_left = int(clamp(math.floor(min(xs)), 0, width - 1))
        raw_right = int(clamp(math.ceil(max(xs)), 0, width - 1))
        raw_top = int(clamp(math.floor(min(ys)), 0, height - 1))
        raw_bottom = int(clamp(math.ceil(max(ys)), 0, height - 1))
        left = max(0, raw_left - 8)
        right = min(width - 1, raw_right + 8)
        top = max(0, raw_top - 8)
        bottom = min(height - 1, raw_bottom + 8)

        yy, xx = np.mgrid[top:bottom + 1, left:right + 1]
        px = xx + 0.5
        py = yy + 0.5
        window = rgba[top:bottom + 1, left:right + 1]
        alpha_ok = window[:, :, 3] >= 220
        inside = points_in_polygon(px, py, geometry["points"])
        support_mask = inside & alpha_ok
        if not support_mask.any():
            continue

        distance_to_edge = edge_distances(px, py, geometry["points"])
        background_mask = (~inside) & alpha_ok & (distance_to_edge <= 7)
        window_rgb = window[:, :, :3].astype(np.float64)
        if background_mask.any():
            background_pixels = window_rgb[background_mask]
        else:
            border = support_mask & ((xx == raw_left) | (xx == raw_right) | (yy == raw_top) | (yy == raw_bottom))
            background_pixels = window_rgb[border] if border.any() else np.zeros((1, 3))
        background_color = np.median(background_pixels, axis=0)

        support_rgb = window_rgb[support_mask]
        distances = np.sqrt(((support_rgb - background_color) ** 2).sum(axis=1))
        threshold = max(22.0, float(np.percentile(distances, 62)) * 0.48)
        selected = distances >= threshold
        if int(selected.sum()) < max(12, round(support_rgb.shape[0] * 0.008)):
            selected = np.ones(support_rgb.shape[0], dtype=bool)

        selected_distances = distances[selected]
        core_threshold = float(np.percentile(selected_distances, 85)) if selected_distances.size else 0.0
        core = selected_distances >= core_threshold
        foreground_pixels = support_rgb[selected][core] if core.any() else support_rgb[selected]
        foreground_colors[-1] = rgb_hex(np.median(foreground_pixels, axis=0))

        selected_2d = np.zeros(support_mask.shape, dtype=bool)
        selected_2d[support_mask] = selected
        sub_mask = mask[top:bottom + 1, left:right + 1]
        masked_pixel_count += int((selected_2d & (sub_mask == 0)).sum())
        sub_mask[selected_2d] = 255

    return {
        "mask": mask,
        "foregroundColors": foreground_colors,
        "maskedPixelCount": masked_pixel_count,
        "regionCount": len(regions),
    }


# ---------------------------------------------------------------------------
# 第 3 步：擦字（Big LaMa 直连 / UniPPT 代理 / 本地回退）
# ---------------------------------------------------------------------------

def inpaint_via_clean_image(url: str, key: str, image_bytes: bytes, mask_bytes: bytes) -> Tuple[bytes, Dict[str, Any]]:
    endpoint = url.rstrip("/")
    if not endpoint.endswith("/v1/inpaint"):
        endpoint = endpoint + ("/inpaint" if endpoint.endswith("/v1") else "/v1/inpaint")
    response = requests.post(
        endpoint,
        headers={"Authorization": f"Bearer {key}"},
        files={
            "image": ("source.png", image_bytes, "image/png"),
            "mask": ("mask.png", mask_bytes, "image/png"),
        },
        data={"mask_dilate": "6", "feather_radius": "0", "context_margin": "128", "roi_mode": "auto"},
        timeout=(12, 300),
    )
    if response.status_code != 200:
        raise RuntimeError(f"clean-image HTTP {response.status_code}：{response.text[:400]}")
    if not response.content:
        raise RuntimeError("clean-image 返回空图片")
    return response.content, {
        "strategy": "big-lama-roi-v1",
        "provider": "clean-image-direct",
        "modelMs": response.headers.get("X-Model-Inference-Ms"),
    }


def inpaint_via_unippt_server(server: str, image_data_url: str, mask_data_url: str) -> Tuple[bytes, Dict[str, Any]]:
    response = requests.post(
        server.rstrip("/") + "/api/ai/inpaint",
        json={
            "image": image_data_url,
            "mask": mask_data_url,
            "maskDilate": 6,
            "featherRadius": 0,
            "contextMargin": 128,
            "roiMode": "auto",
        },
        timeout=(6, 300),
    )
    if response.status_code != 200:
        raise RuntimeError(f"UniPPT /api/ai/inpaint HTTP {response.status_code}：{response.text[:400]}")
    data = response.json()
    cleaned = data.get("image", "")
    if not cleaned.startswith("data:image/"):
        raise RuntimeError("UniPPT 服务未返回有效图片")
    cleaning = data.get("cleaning") or {}
    return decode_data_url(cleaned), {
        "strategy": cleaning.get("strategy", "server"),
        "provider": "unippt-server",
        "modelMs": (cleaning.get("timings") or {}).get("modelMs"),
    }


def dilate_mask(mask_bool: np.ndarray, iterations: int = 3) -> np.ndarray:
    out = mask_bool.copy()
    for _ in range(iterations):
        grown = out.copy()
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            grown |= np.roll(out, (dy, dx), axis=(0, 1))
        out = grown
    return out


def _blur_masked(rgb: np.ndarray, mask_bool: np.ndarray, rounds: int = 4) -> np.ndarray:
    out = rgb.astype(np.float32)
    for _ in range(rounds):
        accumulator = out.copy()
        count = np.ones(out.shape[:2], dtype=np.float32)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            accumulator += np.roll(out, (dy, dx), axis=(0, 1))
            count += 1
        smoothed = accumulator / count[..., None]
        out[mask_bool] = smoothed[mask_bool]
    return np.clip(out, 0, 255).astype(np.uint8)


def inpaint_local(rgba: np.ndarray, mask: np.ndarray) -> Tuple[np.ndarray, str]:
    """离线回退：OpenCV TELEA → scipy 最近邻填充 → 纯 numpy 迭代填充。"""
    unknown = dilate_mask(mask > 0, iterations=3)
    rgb = rgba[:, :, :3]
    try:
        import cv2  # type: ignore
        repaired = cv2.inpaint(rgb[:, :, ::-1].copy(), unknown.astype(np.uint8) * 255, 5, cv2.INPAINT_TELEA)
        result = rgba.copy()
        result[:, :, :3] = repaired[:, :, ::-1]
        return result, "local-opencv-telea"
    except Exception:
        pass
    try:
        from scipy import ndimage  # type: ignore
        _, indices = ndimage.distance_transform_edt(unknown, return_indices=True)
        filled = rgb[indices[0], indices[1]]
        merged = rgb.copy()
        merged[unknown] = filled[unknown]
        result = rgba.copy()
        result[:, :, :3] = _blur_masked(merged, unknown)
        return result, "local-scipy-nearest"
    except Exception:
        pass
    # 纯 numpy：从边缘向内迭代平均填充
    known = ~unknown
    out = rgb.astype(np.float32)
    out[unknown] = 0.0
    for _ in range(max(rgba.shape[0], rgba.shape[1])):
        if known.all():
            break
        accumulator = np.zeros_like(out)
        count = np.zeros(out.shape[:2], dtype=np.float32)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            rolled = np.roll(out, (dy, dx), axis=(0, 1))
            rolled_known = np.roll(known, (dy, dx), axis=(0, 1))
            accumulator += rolled * rolled_known[..., None]
            count += rolled_known
        newly = (~known) & (count > 0)
        if not newly.any():
            break
        out[newly] = accumulator[newly] / count[newly][..., None]
        known |= newly
    result = rgba.copy()
    result[:, :, :3] = _blur_masked(np.clip(out, 0, 255).astype(np.uint8), unknown, rounds=2)
    return result, "local-numpy-diffusion"


def run_inpaint(mode: str, rgba: np.ndarray, mask: np.ndarray, source_png: bytes,
                image_data_url: str, clean_url: str, clean_key: str,
                server: str) -> Tuple[Image.Image, Dict[str, Any]]:
    mask_image = Image.fromarray(np.stack([mask, mask, mask], axis=-1), mode="RGB")
    mask_png = png_bytes(mask_image)
    mask_data_url = encode_data_url(mask_png, "image/png")
    attempts: List[str] = []
    if mode == "auto":
        if clean_url and clean_key:
            attempts.append("direct")
        attempts.extend(["server", "local"])
    else:
        attempts.append(mode)

    errors: List[str] = []
    for attempt in attempts:
        try:
            if attempt == "direct":
                if not clean_url or not clean_key:
                    raise RuntimeError("未配置 UNIPPT_CLEAN_IMAGE_URL / UNIPPT_CLEAN_IMAGE_KEY")
                print("  → 尝试 clean-image 直连（Big LaMa）…")
                cleaned_bytes, meta = inpaint_via_clean_image(clean_url, clean_key, source_png, mask_png)
            elif attempt == "server":
                print(f"  → 尝试 UniPPT 服务器代理 {server} …")
                cleaned_bytes, meta = inpaint_via_unippt_server(server, image_data_url, mask_data_url)
            elif attempt == "local":
                print("  → 使用本地回退填充（质量低于 Big LaMa）…")
                repaired, strategy = inpaint_local(rgba, mask)
                return Image.fromarray(repaired, mode="RGBA"), {"strategy": strategy, "provider": "local"}
            else:
                raise RuntimeError(f"未知擦字模式 {attempt}")
            cleaned = Image.open(io.BytesIO(cleaned_bytes)).convert("RGBA")
            if cleaned.size != (rgba.shape[1], rgba.shape[0]):
                cleaned = cleaned.resize((rgba.shape[1], rgba.shape[0]), Image.LANCZOS)
            return cleaned, meta
        except Exception as error:  # noqa: BLE001 — 逐级降级，记录原因
            message = f"{attempt}: {error}"
            errors.append(message)
            print(f"    ✗ {message}")
    raise RuntimeError("全部擦字通道失败：" + "；".join(errors))


# ---------------------------------------------------------------------------
# 第 4 步：可编辑 HTML 生成
# ---------------------------------------------------------------------------

def _cjk_ratio(text: str) -> float:
    if not text:
        return 0.0
    cjk = sum(1 for ch in text if "\u2e80" <= ch <= "\u9fff" or "\uf900" <= ch <= "\ufaff")
    return cjk / len(text)


def region_box(region: Dict[str, Any], width: int, height: int) -> Optional[Dict[str, float]]:
    geometry = region_polygon(region, width, height)
    if not geometry:
        return None
    xs = [p[0] for p in geometry["points"]]
    ys = [p[1] for p in geometry["points"]]
    return {
        "left": min(xs), "top": min(ys),
        "width": max(1.0, max(xs) - min(xs)), "height": max(1.0, max(ys) - min(ys)),
        "shortEdge": geometry["shortEdge"],
    }


def estimate_font_size(text: str, box: Dict[str, float], vertical: bool) -> int:
    glyphs = max(1, len(text.replace(" ", "")))
    line_extent = box["width"] if vertical else box["height"]
    run_extent = box["height"] if vertical else box["width"]
    estimate = line_extent * 0.82
    per_glyph = 1.0 if _cjk_ratio(text) >= 0.4 else 1.75
    estimate = min(estimate, run_extent / glyphs * per_glyph)
    return int(clamp(round(estimate), 8, 512))


def fallback_color(cleaned_rgba: np.ndarray, box: Dict[str, float]) -> str:
    left, top = int(box["left"]), int(box["top"])
    right = min(cleaned_rgba.shape[1], left + int(box["width"]) + 1)
    bottom = min(cleaned_rgba.shape[0], top + int(box["height"]) + 1)
    patch = cleaned_rgba[top:bottom, left:right, :3]
    if patch.size == 0:
        return "#111111"
    luminance = float((patch.astype(np.float32) * np.array([0.2126, 0.7152, 0.0722])).sum(axis=-1).mean())
    return "#111111" if luminance > 140 else "#f5f5f5"


def build_text_items(regions: List[Dict[str, Any]], colors: List[Optional[str]],
                     cleaned_rgba: np.ndarray, width: int, height: int) -> str:
    items: List[str] = []
    for index, region in enumerate(regions):
        text = str(region.get("text", "")).strip()
        box = region_box(region, width, height)
        if not text or not box:
            continue
        vertical = box["height"] > box["width"] * 1.6 and len(text) >= 2
        font_size = estimate_font_size(text, box, vertical)
        color = colors[index] if index < len(colors) and colors[index] else fallback_color(cleaned_rgba, box)
        style = (
            f"left:{box['left']:.1f}px;top:{box['top']:.1f}px;"
            f"width:{box['width']:.1f}px;height:{box['height']:.1f}px;"
            f"font-size:{font_size}px;color:{color};"
        )
        if vertical:
            style += "writing-mode:vertical-rl;"
        else:
            style += f"line-height:{box['height']:.1f}px;"
        items.append(
            f'  <div class="txt" contenteditable="true" data-index="{index}" style="{style}">'
            f"{html_module.escape(text)}</div>"
        )
    return "\n".join(items)


HTML_TEMPLATE = """<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>__TITLE__ · 可编辑重建</title>
<style>
  * { box-sizing: border-box; }
  body { margin: 0; font-family: "Segoe UI", "Microsoft YaHei", sans-serif; background: #202124; color: #e8eaed; }
  header { position: sticky; top: 0; z-index: 50; display: flex; flex-wrap: wrap; gap: 8px; align-items: center;
           padding: 10px 14px; background: #2d2e31; box-shadow: 0 1px 4px rgba(0,0,0,.4); }
  header .title { font-weight: 600; margin-right: 8px; }
  header button, header select { border: 1px solid #5f6368; background: #3c4043; color: #e8eaed;
           border-radius: 16px; padding: 5px 12px; cursor: pointer; font-size: 13px; }
  header button:hover { background: #4a4e52; }
  header .status { margin-left: auto; font-size: 12px; color: #9aa0a6; }
  #viewport { padding: 24px; overflow: auto; height: calc(100vh - 56px); }
  #sizer { position: relative; }
  #stage { position: absolute; left: 0; top: 0; width: __WIDTH__px; height: __HEIGHT__px; transform-origin: 0 0;
           background: repeating-conic-gradient(#3c4043 0 25%, #2d2e31 0 50%) 0 0 / 24px 24px;
           box-shadow: 0 4px 24px rgba(0,0,0,.5); }
  #stage img.bg { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; user-select: none; }
  .txt { position: absolute; margin: 0; padding: 0; white-space: pre; outline: none; cursor: text;
         min-width: 8px; min-height: 8px; }
  body.show-outline .txt { outline: 1px dashed rgba(66,133,244,.9); }
  .txt.selected { outline: 2px solid #fbbc04 !important; }
  body.move-mode .txt { cursor: move; user-select: none; }
</style>
</head>
<body class="show-outline">
<header>
  <span class="title">__TITLE__</span>
  <select id="bg-mode" title="背景图层">
    <option value="clean" selected>擦字背景</option>
    <option value="original">原图</option>
    <option value="none">无背景</option>
  </select>
  <button id="mode-toggle" type="button">模式：编辑文字</button>
  <button id="outline-toggle" type="button">边框：开</button>
  <button id="add-text" type="button">添加文本</button>
  <button id="delete-selected" type="button" disabled>删除选中</button>
  <button id="zoom-out" type="button">−</button>
  <button id="zoom-fit" type="button">适配</button>
  <button id="zoom-in" type="button">＋</button>
  <button id="export-html" type="button">导出 HTML</button>
  <button id="export-json" type="button">导出 JSON</button>
  <span class="status">__STATUS__</span>
</header>
<div id="viewport"><div id="sizer"><div id="stage">
  <img class="bg" id="bg-clean" src="__BG_CLEAN__" alt="">
  <img class="bg" id="bg-original" src="__BG_ORIGINAL__" alt="" style="display:none">
__ITEMS__
</div></div></div>
<script>
(function () {
  "use strict";
  var stage = document.getElementById("stage");
  var sizer = document.getElementById("sizer");
  var viewport = document.getElementById("viewport");
  var stageWidth = __WIDTH__, stageHeight = __HEIGHT__;
  var zoom = 1, selected = null, moveMode = false;

  function applyZoom(value) {
    zoom = Math.min(8, Math.max(0.05, value));
    stage.style.transform = "scale(" + zoom + ")";
    sizer.style.width = (stageWidth * zoom) + "px";
    sizer.style.height = (stageHeight * zoom) + "px";
  }
  function fit() {
    applyZoom(Math.min((viewport.clientWidth - 48) / stageWidth, (viewport.clientHeight - 48) / stageHeight, 1));
  }
  function select(item) {
    if (selected) selected.classList.remove("selected");
    selected = item || null;
    if (selected) selected.classList.add("selected");
    document.getElementById("delete-selected").disabled = !selected;
  }
  function setMode(move) {
    moveMode = move;
    document.body.classList.toggle("move-mode", move);
    document.querySelectorAll(".txt").forEach(function (el) {
      el.setAttribute("contenteditable", move ? "false" : "true");
    });
    document.getElementById("mode-toggle").textContent = move ? "模式：移动布局" : "模式：编辑文字";
  }
  stage.addEventListener("pointerdown", function (event) {
    var item = event.target.closest(".txt");
    select(item);
    if (!item || !moveMode) return;
    event.preventDefault();
    var startX = event.clientX, startY = event.clientY;
    var left0 = parseFloat(item.style.left) || 0, top0 = parseFloat(item.style.top) || 0;
    function onMove(ev) {
      item.style.left = (left0 + (ev.clientX - startX) / zoom) + "px";
      item.style.top = (top0 + (ev.clientY - startY) / zoom) + "px";
    }
    function onUp() {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", onUp);
    }
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp);
  });
  document.getElementById("bg-mode").addEventListener("change", function () {
    var mode = this.value;
    document.getElementById("bg-clean").style.display = mode === "clean" ? "" : "none";
    document.getElementById("bg-original").style.display = mode === "original" ? "" : "none";
  });
  if (!document.getElementById("bg-original").getAttribute("src")) {
    document.querySelector('#bg-mode option[value="original"]').disabled = true;
  }
  document.getElementById("mode-toggle").onclick = function () { setMode(!moveMode); };
  document.getElementById("outline-toggle").onclick = function () {
    var on = document.body.classList.toggle("show-outline");
    this.textContent = on ? "边框：开" : "边框：关";
  };
  document.getElementById("add-text").onclick = function () {
    var div = document.createElement("div");
    div.className = "txt";
    div.setAttribute("contenteditable", moveMode ? "false" : "true");
    div.style.cssText = "left:" + (viewport.scrollLeft / zoom + 40) + "px;top:" +
      (viewport.scrollTop / zoom + 40) + "px;font-size:32px;color:#f5f5f5;line-height:1.2;";
    div.textContent = "双击编辑文字";
    stage.appendChild(div);
    select(div);
  };
  document.getElementById("delete-selected").onclick = function () {
    if (selected) { selected.remove(); select(null); }
  };
  document.getElementById("zoom-in").onclick = function () { applyZoom(zoom * 1.15); };
  document.getElementById("zoom-out").onclick = function () { applyZoom(zoom / 1.15); };
  document.getElementById("zoom-fit").onclick = fit;
  function download(name, type, content) {
    var blob = new Blob([content], { type: type });
    var link = document.createElement("a");
    link.href = URL.createObjectURL(blob);
    link.download = name;
    link.click();
    setTimeout(function () { URL.revokeObjectURL(link.href); }, 5000);
  }
  document.getElementById("export-html").onclick = function () {
    select(null);
    download(document.title + ".edited.html", "text/html",
      "<!DOCTYPE html>\\n" + document.documentElement.outerHTML);
  };
  document.getElementById("export-json").onclick = function () {
    var items = [];
    document.querySelectorAll(".txt").forEach(function (el) {
      items.push({
        text: el.innerText,
        left: parseFloat(el.style.left) || 0,
        top: parseFloat(el.style.top) || 0,
        width: parseFloat(el.style.width) || null,
        height: parseFloat(el.style.height) || null,
        fontSize: parseFloat(el.style.fontSize) || null,
        color: el.style.color || null,
        vertical: el.style.writingMode === "vertical-rl",
      });
    });
    download(document.title + ".items.json", "application/json",
      JSON.stringify({ width: stageWidth, height: stageHeight, items: items }, null, 2));
  };
  window.addEventListener("resize", fit);
  fit();
})();
</script>
</body>
</html>
"""


def build_editable_html(title: str, width: int, height: int, clean_data_url: str,
                        original_data_url: str, items_html: str, status: str) -> str:
    return (HTML_TEMPLATE
            .replace("__TITLE__", html_module.escape(title))
            .replace("__WIDTH__", str(int(width)))
            .replace("__HEIGHT__", str(int(height)))
            .replace("__BG_CLEAN__", clean_data_url)
            .replace("__BG_ORIGINAL__", original_data_url)
            .replace("__ITEMS__", items_html)
            .replace("__STATUS__", html_module.escape(status)))


# ---------------------------------------------------------------------------
# 主流程
# ---------------------------------------------------------------------------

def convert(args: argparse.Namespace) -> Path:
    input_path = Path(args.image)
    if not input_path.is_file():
        raise RuntimeError(f"图片文件不存在：{input_path}")
    output_dir = Path(args.output) if args.output else input_path.parent / f"{input_path.stem}-editable"
    output_dir.mkdir(parents=True, exist_ok=True)

    env_ai = find_env_values(".env.local")
    env_clean = find_env_values(".env.clean-image.local")
    api_key = resolve_setting(args.api_key, ["UNIPPT_OCR_KEY", "UNIPPT_AI_KEY", "UNIDOC_AI_KEY"], env_ai)
    api_base = resolve_setting(args.api_base, ["UNIPPT_OCR_BASE", "UNIPPT_AI_BASE"], env_ai, DEFAULT_OCR_BASE)
    ocr_model = resolve_setting(args.ocr_model, ["UNIPPT_OCR_MODEL", "UNIDOC_OCR_MODEL"], env_ai, DEFAULT_OCR_MODEL)
    clean_url = resolve_setting(args.clean_url, ["UNIPPT_CLEAN_IMAGE_URL"], env_clean)
    clean_key = resolve_setting(args.clean_key, ["UNIPPT_CLEAN_IMAGE_KEY"], env_clean)

    image = Image.open(input_path).convert("RGBA")
    width, height = image.size
    rgba = np.array(image, dtype=np.uint8)
    source_bytes = input_path.read_bytes()
    source_mime = MIME_BY_SUFFIX.get(input_path.suffix.lower(), "image/png")
    image_data_url = encode_data_url(source_bytes, source_mime)
    print(f"输入：{input_path}（{width}x{height}）")

    # 1) OCR
    ocr_cache = output_dir / "ocr.json"
    if args.reuse_ocr and ocr_cache.is_file():
        detection = json.loads(ocr_cache.read_text(encoding="utf-8"))
        print(f"步骤 1/4 OCR：复用缓存 {ocr_cache}（{detection.get('regionCount', 0)} 个文字区）")
    else:
        if not api_key:
            raise RuntimeError("缺少 OCR API Key：请用 --api-key，或配置 UNIPPT_AI_KEY（.env.local）")
        print(f"步骤 1/4 OCR：调用 {ocr_model} …")
        detection = run_ocr(image_data_url, width, height, input_path.name, api_key, api_base, ocr_model)
        ocr_cache.write_text(json.dumps(detection, ensure_ascii=False, indent=2), encoding="utf-8")
        print(f"  → {detection['regionCount']} 个文字区，request_id={detection.get('requestId')}")
    regions = detection.get("regions") or []
    if not regions:
        raise RuntimeError("OCR 结果没有文字区域")

    # 2) 掩码
    print("步骤 2/4 掩码：按颜色对比度选中文字像素…")
    mask_result = build_text_mask(rgba, regions)
    mask = mask_result["mask"]
    print(f"  → 掩码像素 {mask_result['maskedPixelCount']}，区域 {mask_result['regionCount']}")
    if mask_result["maskedPixelCount"] == 0:
        raise RuntimeError("文字掩码为空：OCR 坐标可能与图片不匹配")
    mask_path = output_dir / "mask.png"
    Image.fromarray(np.stack([mask, mask, mask], axis=-1), mode="RGB").save(mask_path)

    # 3) 擦字
    print(f"步骤 3/4 擦字（模式 {args.inpaint}）：")
    cleaned_image, cleaning = run_inpaint(
        args.inpaint, rgba, mask, png_bytes(image), image_data_url,
        clean_url, clean_key, args.server,
    )
    cleaned_path = output_dir / "cleaned.png"
    cleaned_image.save(cleaned_path)
    print(f"  → 策略 {cleaning.get('strategy')}（{cleaning.get('provider')}），已存 {cleaned_path}")

    # 4) HTML
    print("步骤 4/4 生成可编辑 HTML…")
    cleaned_rgba = np.array(cleaned_image, dtype=np.uint8)
    items_html = build_text_items(regions, mask_result["foregroundColors"], cleaned_rgba, width, height)
    cleaned_png = png_bytes(cleaned_image)
    if len(cleaned_png) > 4 * 1024 * 1024:
        buffer = io.BytesIO()
        cleaned_image.convert("RGB").save(buffer, format="JPEG", quality=92)
        clean_data_url = encode_data_url(buffer.getvalue(), "image/jpeg")
    else:
        clean_data_url = encode_data_url(cleaned_png, "image/png")
    original_data_url = "" if args.no_original else image_data_url
    status = (f"{detection.get('regionCount', 0)} 个文本框 · 擦字 {cleaning.get('strategy')} · "
              f"OCR {detection.get('model', '')}")
    document = build_editable_html(input_path.stem, width, height, clean_data_url,
                                   original_data_url, items_html, status)
    html_path = output_dir / f"{input_path.stem}.editable.html"
    html_path.write_text(document, encoding="utf-8")
    print(f"\n完成：{html_path}")
    print(f"调试产物：{ocr_cache.name} / {mask_path.name} / {cleaned_path.name}（同目录）")
    return html_path


# ---------------------------------------------------------------------------
# 离线自检（不访问网络）
# ---------------------------------------------------------------------------

def self_test() -> None:
    # 与 web/ai_attachment_runtime.test.js 的合成夹具一致：黑底 + 白色文字块
    width, height = 40, 20
    rgba = np.zeros((height, width, 4), dtype=np.uint8)
    rgba[..., 3] = 255
    rgba[8:13, 10:20, :3] = 255
    regions = [{"text": "TEST", "location": [5, 5, 35, 5, 35, 15, 5, 15]}]
    result = build_text_mask(rgba, regions)
    assert result["mask"][9, 12] == 255, "文字像素应被选入掩码"
    assert result["mask"][9, 30] == 0, "背景像素不应进掩码"
    assert result["maskedPixelCount"] == 50, f"掩码像素应为 50，实际 {result['maskedPixelCount']}"
    assert result["foregroundColors"][0] == "#ffffff", f"前景色应为 #ffffff，实际 {result['foregroundColors'][0]}"

    repaired, strategy = inpaint_local(rgba, result["mask"])
    assert repaired[9, 12, :3].max() <= 60, f"擦除后文字区应接近黑色背景（策略 {strategy}）"

    cleaned_rgba = repaired
    items = build_text_items(regions, result["foregroundColors"], cleaned_rgba, width, height)
    assert 'contenteditable="true"' in items and "TEST" in items
    document = build_editable_html("self-test", width, height, "data:image/png;base64,AAAA", "", items, "ok")
    assert "__TITLE__" not in document and "contenteditable" in document

    data_url = encode_data_url(b"\x89PNG", "image/png")
    assert decode_data_url(data_url) == b"\x89PNG"
    assert rgb_hex((255, 255, 255)) == "#ffffff" and rgb_hex((0, 128.4, 300)) == "#0080ff"
    assert dashscope_ocr_url("https://dashscope.aliyuncs.com/compatible-mode/v1") == (
        "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation")
    print(f"SELF-TEST OK（本地擦字策略：{strategy}）")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="图片转可编辑 HTML（OCR + 文字掩码 + Big LaMa 擦字 + contenteditable 重建）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="示例：\n"
               "  python image_to_editable_html.py poster.png\n"
               "  python image_to_editable_html.py poster.png -o out --inpaint direct\n"
               "  python image_to_editable_html.py poster.png --inpaint local   # 完全离线（已有 ocr.json 时配合 --reuse-ocr）\n")
    parser.add_argument("image", nargs="?", help="输入图片路径")
    parser.add_argument("-o", "--output", help="输出目录（默认 <图片名>-editable/）")
    parser.add_argument("--api-key", help="DashScope API Key（默认读环境变量或 .env.local）")
    parser.add_argument("--api-base", help=f"OCR 接口地址（默认 {DEFAULT_OCR_BASE}）")
    parser.add_argument("--ocr-model", help=f"OCR 模型名（默认 {DEFAULT_OCR_MODEL}）")
    parser.add_argument("--inpaint", choices=["auto", "direct", "server", "local"], default="auto",
                        help="擦字通道：auto=直连→服务器代理→本地（默认）")
    parser.add_argument("--clean-url", help="clean-image 服务地址（默认读 .env.clean-image.local）")
    parser.add_argument("--clean-key", help="clean-image 服务密钥（默认读 .env.clean-image.local）")
    parser.add_argument("--server", default=DEFAULT_UNIPPT_SERVER,
                        help=f"UniPPT 服务器地址，用于 /api/ai/inpaint 代理（默认 {DEFAULT_UNIPPT_SERVER}）")
    parser.add_argument("--reuse-ocr", action="store_true", help="输出目录已有 ocr.json 时直接复用，不再调 OCR")
    parser.add_argument("--no-original", action="store_true", help="HTML 中不内嵌原图图层（减小体积）")
    parser.add_argument("--self-test", action="store_true", help="运行离线自检后退出")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        return
    if not args.image:
        parser.error("缺少输入图片路径（或使用 --self-test）")
    try:
        convert(args)
    except Exception as error:  # noqa: BLE001
        print(f"\n✗ 处理失败：{error}")
        sys.exit(1)


if __name__ == "__main__":
    main()
