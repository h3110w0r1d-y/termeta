"""沿第一层描边外沿连接圆角公共切线，生成连续厚度轮廓及亮色底图稿。"""

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parent.parent
BRAND = ROOT / "resources/brand"
NS = "http://www.w3.org/2000/svg"
ET.register_namespace("", NS)
DEPTH = (18., -11.16)
OFFSET_TOLERANCE = 0.00001


def add(a, b):
    return tuple(x + y for x, y in zip(a, b))


def mix(a, b, t):
    return tuple(x * (1-t) + y * t for x, y in zip(a, b))


def cross(a, b):
    return a[0]*b[1] - a[1]*b[0]


def curve_value(curve, t):
    points = curve
    while len(points) > 1:
        points = [mix(points[i], points[i+1], t) for i in range(len(points)-1)]
    return points[0]


def offset_point(point, tangent, radius):
    length = math.hypot(*tangent)
    # 轮廓在 SVG 坐标中顺时针；右法向指向填充区外侧。
    return add(point, (radius*tangent[1]/length, -radius*tangent[0]/length))


def offset_curve(curve, radius):
    derivative = [tuple(3*(b[i]-a[i]) for i in range(2))
                  for a, b in zip(curve, curve[1:])]
    second = [tuple(2*(b[i]-a[i]) for i in range(2))
              for a, b in zip(derivative, derivative[1:])]

    def sample(t):
        velocity = curve_value(derivative, t)
        acceleration = curve_value(second, t)
        speed = math.hypot(*velocity)
        factor = 1 + radius*cross(velocity, acceleration)/speed**3
        assert factor > 0
        return (offset_point(curve_value(curve, t), velocity, radius),
                tuple(v*factor for v in velocity))

    def fit(start, end, depth=0):
        p0, v0 = sample(start)
        p3, v3 = sample(end)
        step = (end-start)/3
        candidate = [p0, add(p0, tuple(v*step for v in v0)),
                     add(p3, tuple(-v*step for v in v3)), p3]
        errors = [math.dist(curve_value(candidate, t), sample(start+(end-start)*t)[0])
                  for t in (0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875)]
        if max(errors) <= OFFSET_TOLERANCE:
            return [candidate], max(errors)
        assert depth < 16, "圆角外扩拟合未收敛"
        left, left_error = fit(start, (start+end)/2, depth+1)
        right, right_error = fit((start+end)/2, end, depth+1)
        return left+right, max(left_error, right_error)

    # 三次贝塞尔的等距线通常不是三次曲线；自适应细分拟合，同时保持端点切向。
    return fit(0, 1)


def split(curve, t):
    a = [mix(curve[i], curve[i+1], t) for i in range(3)]
    b = [mix(a[i], a[i+1], t) for i in range(2)]
    p = mix(b[0], b[1], t)
    return [curve[0], a[0], b[0], p], [p, b[1], a[2], curve[3]]


def tangent_parameter(curve):
    # 三次贝塞尔导数为二次式；求导数与厚度方向叉积为零的根。
    p0, p1, p2, p3 = curve
    coefficients = [
        tuple(-p0[i]+3*p1[i]-3*p2[i]+p3[i] for i in range(2)),
        tuple(2*(p0[i]-2*p1[i]+p2[i]) for i in range(2)),
        tuple(p1[i]-p0[i] for i in range(2)),
    ]
    a, b, c = [cross(v, DEPTH) for v in coefficients]
    if abs(a) < 1e-12:
        roots = [-c/b]
    else:
        discriminant = b*b-4*a*c
        assert discriminant >= 0
        roots = [(-b+math.sqrt(discriminant))/(2*a),
                 (-b-math.sqrt(discriminant))/(2*a)]
    roots = [t for t in roots if 0 < t < 1]
    assert len(roots) == 1
    t = roots[0]
    derivative = tuple(3*(coefficients[0][i]*t*t+coefficients[1][i]*t+coefficients[2][i]) for i in range(2))
    error = abs(cross(derivative, DEPTH))/(math.hypot(*derivative)*math.hypot(*DEPTH))
    assert error < 1e-12
    return t, error


def segments(path):
    tokens = re.findall(r"[MLCZ]|[-+]?(?:\d*\.\d+|\d+)(?:[eE][-+]?\d+)?", path)
    result = []
    current = None
    i = 0
    while i < len(tokens):
        command = tokens[i]
        i += 1
        if command == "Z":
            break
        count = {"M": 2, "L": 2, "C": 6}[command]
        values = [float(x) for x in tokens[i:i+count]]
        points = list(zip(values[::2], values[1::2]))
        i += count
        if command != "M":
            result.append((command, [current, *points]))
        current = points[-1]
    assert [op for op, _ in result] == ["L", "C"]*4
    return result


def body_path(front, stroke_width):
    outline = segments(front)
    tl_t, tl_error = tangent_parameter(outline[7][1])
    br_t, br_error = tangent_parameter(outline[3][1])
    tl_before, tl_after = split(outline[7][1], tl_t)
    br_before, br_after = split(outline[3][1], br_t)
    result = []
    radius = stroke_width/2
    max_error = 0
    max_join_gap = 0
    current = None

    def emit(command, points, depth=False):
        if depth:
            points = [add(p, DEPTH) for p in points]
        result.append(command + " " + " ".join(f"{x:.6f} {y:.6f}" for x, y in points))

    def offset_emit(command, points, depth=False):
        nonlocal max_error, max_join_gap, current
        if command == "C":
            curves, error = offset_curve(points, radius)
            max_error = max(max_error, error)
            start = curves[0][0]
        else:
            direction = tuple(b-a for a, b in zip(points[0], points[1]))
            start, end = [offset_point(p, direction, radius) for p in points]
            curves = None
        placed_start = add(start, DEPTH) if depth else start
        if current is None:
            emit("M", [start], depth)
        else:
            max_join_gap = max(max_join_gap, math.dist(current, placed_start))
        if curves:
            for curve in curves:
                emit("C", curve[1:], depth)
            end = curves[-1][-1]
        else:
            emit("L", [end], depth)
        current = add(end, DEPTH) if depth else end

    tl_outer = offset_point(tl_after[0], tuple(b-a for a, b in zip(tl_after[0], tl_after[1])), radius)
    br_outer = offset_point(br_after[0], tuple(b-a for a, b in zip(br_after[0], br_after[1])), radius)
    # 底面没有描边，直接扩大其填充轮廓；连接线从正面描边外沿出发。
    # 前后圆角等距外扩相同半径，公共切线仍平行于厚度方向。
    emit("M", [tl_outer])
    emit("L", [tl_outer], True)
    current = add(tl_outer, DEPTH)
    for command, points in [("C", tl_after), *outline[:3], ("C", br_before)]:
        offset_emit(command, points, True)
    emit("L", [br_outer])
    current = br_outer
    for command, points in [("C", br_after), *outline[4:7], ("C", tl_before)]:
        offset_emit(command, points)
    max_join_gap = max(max_join_gap, math.dist(current, tl_outer))
    # 原图坐标仅保留三位小数，切向舍入造成的接点误差随外扩半径同比增长。
    assert radius > 0 and max_join_gap < radius*2*OFFSET_TOLERANCE
    result.append("Z")
    return " ".join(result), {
        "depth_vector": DEPTH,
        "front_stroke_width": stroke_width,
        "body_outward_offset": radius,
        "offset_fit_max_sample_error": max_error,
        "offset_join_max_gap": max_join_gap,
        "top_left": {"t": tl_t, "front_centerline": tl_after[0], "front_outer": tl_outer, "back_outer": add(tl_outer, DEPTH), "tangent_cross_error": tl_error},
        "bottom_right": {"t": br_t, "front_centerline": br_after[0], "front_outer": br_outer, "back_outer": add(br_outer, DEPTH), "tangent_cross_error": br_error},
    }


def write(tree, path):
    tree.write(path, encoding="utf-8", xml_declaration=True)


def refine(report_path):
    report = None
    for name in ["termeta-icon.svg", "termeta-symbol.svg"]:
        tree = ET.parse(BRAND / name)
        root = tree.getroot()
        group = next(e for e in root.iter() if e.get("id") == "panel-front")
        face = next(e for e in group if e.get("fill") == "url(#screen)")
        body = next(e for e in group if e.get("fill") == "url(#bevel)")
        rim = next(e for e in group if e.get("stroke") == "url(#screenRim)")
        original_face = face.get("d")
        assert rim.get("d") == original_face
        body_data, geometry = body_path(original_face, float(rim.get("stroke-width")))
        body.set("d", body_data)
        body.set("id", "front-panel-body")
        assert face.get("d") == original_face
        write(tree, BRAND / name)
        if report is None:
            report = geometry
        else:
            assert geometry == report

    light = ET.parse(BRAND / "termeta-icon.svg")
    palettes = {
        "tile": ["#ffffff", "#f1f5fb", "#e3ebf6"],
        "tileRim": ["#d9e5f4", "#b6c6dd", "#a6b8cf"],
    }
    for element in light.getroot().iter():
        if element.get("id") in palettes:
            for stop, color in zip(element, palettes[element.get("id")]):
                stop.set("stop-color", color)
        if element.get("id") == "ambient":
            element[0].set("stop-opacity", ".08")
    light.getroot().find(f"{{{NS}}}title").text = "Termeta 三层终端图标 · 亮色底"
    write(light, BRAND / "termeta-icon-light.svg")
    report["files"] = {p.name: hashlib.sha256(p.read_bytes()).hexdigest()
                       for p in sorted(BRAND.glob("*.svg"))}
    if report_path:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"continuous_body": True, "tangent_points": report}, ensure_ascii=False))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", type=Path, help="可选的圆角切线验证报告")
    refine(parser.parse_args().report)
