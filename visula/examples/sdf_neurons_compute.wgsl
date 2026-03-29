struct Neuron {
    pos: vec4f,
    info: vec4f,
};

struct Segment {
    base: vec4f,
    tip: vec4f,
    info: vec4f,
};

struct BVHNode {
    aabb_min: vec4f,
    aabb_max: vec4f,
};

struct Connection {
    data: vec4f,
    info: vec4f,
};

struct Params {
    resolution: vec2f,
    time: f32,
    neuron_count: f32,
    cam_pos: vec4f,
    cam_target: vec4f,
    cam_fov: f32,
    bvh_count: f32,
    conn_count: f32,
    _pad: f32,
};

@group(0) @binding(0) var<storage, read> neurons: array<Neuron>;
@group(0) @binding(1) var<storage, read> segs: array<Segment>;
@group(0) @binding(2) var<storage, read> bvh: array<BVHNode>;
@group(0) @binding(3) var<uniform> P: Params;
@group(0) @binding(4) var out: texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(5) var<storage, read> conns: array<Connection>;

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp(0.5 + 0.5 * (b - a) / k, 0.0, 1.0);
    return mix(b, a, h) - k * h * (1.0 - h);
}

fn sd_tapered_capsule(p: vec3f, a: vec3f, b: vec3f, ra: f32, rb: f32) -> f32 {
    let ba = b - a;
    let pa = p - a;
    let t = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * t) - mix(ra, rb, t);
}

fn sd_capsule(p: vec3f, a: vec3f, b: vec3f, r: f32) -> f32 {
    let ba = b - a;
    let pa = p - a;
    let t = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * t) - r;
}

fn hsh(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453);
}

fn eval_neuron(p: vec3f, ni: i32) -> vec2f {
    let n = neurons[ni];
    let al = n.info.x;
    if (al < 0.01) {
        return vec2f(1e10, 0.0);
    }
    let c = n.pos.xyz;
    let s_r = n.info.y * al;
    var sp = p - c;
    sp.y *= 1.2;
    var dn = length(sp) - s_r;
    let off = i32(n.info.z);
    let cnt = i32(n.info.w);
    for (var i = 0; i < cnt; i++) {
        let s = segs[off + i];
        if (s.base.w < 0.001) {
            continue;
        }
        dn = smin(dn, sd_tapered_capsule(p, s.base.xyz, s.tip.xyz, s.base.w, s.tip.w), s.info.x);
    }
    dn = smin(dn, p.y + 0.005, 0.035);
    return vec2f(dn, select(2.0, 1.0, n.pos.w < 0.5));
}

fn sdf_map(p: vec3f) -> vec2f {
    var d = p.y + 0.005;
    var mat = 0.0;
    let nc = i32(P.neuron_count);
    let bc = i32(P.bvh_count);

    if (bc > 0) {
        var stk: array<i32, 32>;
        var sp = 0;
        stk[0] = 0;
        sp = 1;
        while (sp > 0) {
            sp--;
            let ni = stk[sp];
            if (ni < 0 || ni >= bc) {
                continue;
            }
            let nd = bvh[ni];
            let left = i32(nd.aabb_min.w);
            let right = i32(nd.aabb_max.w);
            let cl = clamp(p, nd.aabb_min.xyz, nd.aabb_max.xyz);
            if (length(p - cl) > d + 0.3) {
                continue;
            }
            if (left == -1) {
                if (right >= 0 && right < nc) {
                    let r = eval_neuron(p, right);
                    if (r.x < d) {
                        d = r.x;
                        mat = r.y;
                    }
                }
            } else {
                if (sp < 30) {
                    stk[sp] = left;
                    sp++;
                }
                if (sp < 30) {
                    stk[sp] = right;
                    sp++;
                }
            }
        }
    }

    let cc = i32(P.conn_count);
    for (var c = 0; c < cc; c++) {
        let cn = conns[c];
        let pr = cn.info.y;
        if (pr < 0.01) {
            continue;
        }
        let f = vec3f(cn.data.x, 0.06, cn.data.y);
        let t = vec3f(cn.data.z, 0.06, cn.data.w);
        let mid2 = (f + t) * 0.5;
        let dir = t - f;
        var prp = vec3f(-dir.z, 0.0, dir.x);
        let pl = length(prp);
        if (pl > 0.001) {
            prp = prp / pl;
        }
        let ca = 0.12 + hsh(vec2f(cn.data.x * 3.0 + cn.data.z, cn.data.y * 5.0 + cn.data.w)) * 0.12;
        let si = select(-1.0, 1.0, hsh(vec2f(cn.data.x + cn.data.z, cn.data.y + cn.data.w)) > 0.5);
        var cp = mid2 + prp * length(dir) * ca * si;
        cp.y = 0.07;
        let ar = 0.012;
        var md = 1e10f;
        for (var i = 0; i < 8; i++) {
            let t3 = f32(i) / 7.0 * pr;
            let ab = mix(f, cp, t3);
            let bc2 = mix(cp, t, t3);
            var pt = mix(ab, bc2, t3);
            pt.y = 0.04 + sin(t3 * 3.14159) * 0.02;
            md = min(md, length(p - pt) - mix(ar * 1.2, ar * 0.4, t3));
        }
        let ab3 = mix(f, cp, pr);
        let bc3 = mix(cp, t, pr);
        var ep = mix(ab3, bc3, pr);
        ep.y = 0.05;
        md = smin(md, length(p - ep) - ar * 2.0, 0.018);
        md = smin(md, p.y + 0.005, 0.012);
        if (md < d) {
            d = md;
            mat = select(4.0, 3.0, cn.info.x < 0.5);
        }
    }
    return vec2f(d, mat);
}

fn calc_normal(p: vec3f) -> vec3f {
    let e = vec2f(0.002, 0.0);
    return normalize(vec3f(
        sdf_map(p + e.xyy).x - sdf_map(p - e.xyy).x,
        sdf_map(p + e.yxy).x - sdf_map(p - e.yxy).x,
        sdf_map(p + e.yyx).x - sdf_map(p - e.yyx).x,
    ));
}

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) gid: vec3u) {
    let dims = vec2f(P.resolution);
    let px = vec2f(f32(gid.x), f32(gid.y));
    if (px.x >= dims.x || px.y >= dims.y) {
        return;
    }

    let uv = vec2f(px.x - 0.5 * dims.x, 0.5 * dims.y - px.y) / dims.y;
    let T = P.time;

    let ro = P.cam_pos.xyz;
    let ta = P.cam_target.xyz;
    let w = normalize(ta - ro);
    let u = normalize(cross(w, vec3f(0.0, 1.0, 0.0)));
    let v = cross(u, w);
    let rd = normalize(u * uv.x + v * uv.y + w * P.cam_fov);

    var bg = mix(vec3f(0.01, 0.012, 0.03), vec3f(0.025, 0.02, 0.045), uv.y + 0.5);

    var t = 0.0;
    var h = vec2f(0.0);
    var hit = false;
    var min_dist = 1e10f;
    var min_mat = 0.0;
    var min_t = 0.0;
    for (var i = 0; i < 50; i++) {
        h = sdf_map(ro + rd * t);
        if (h.x < min_dist) {
            min_dist = h.x;
            min_mat = h.y;
            min_t = t;
        }
        if (h.x < 0.001) {
            hit = true;
            break;
        }
        if (t > 12.0) {
            break;
        }
        t += h.x;
    }

    var col = bg;

    if (min_dist < 0.15) {
        var gc: vec3f;
        if (min_mat < 0.5) {
            gc = vec3f(0.03, 0.04, 0.08);
        } else if (min_mat < 1.5) {
            gc = vec3f(0.15, 0.35, 0.9);
        } else if (min_mat < 2.5) {
            gc = vec3f(0.9, 0.15, 0.1);
        } else if (min_mat < 3.5) {
            gc = vec3f(0.1, 0.3, 0.8);
        } else if (min_mat < 4.5) {
            gc = vec3f(0.8, 0.12, 0.08);
        } else {
            gc = vec3f(0.3, 0.35, 0.4);
        }
        let glow = exp(-min_dist * min_dist * 800.0) * 0.6;
        col += gc * glow;
    }

    if (hit) {
        let p = ro + rd * t;
        let n = calc_normal(p);
        let mt = h.y;
        let V = normalize(ro - p);

        let fresnel = pow(1.0 - abs(dot(n, V)), 2.5);

        var interior: vec3f;
        var edge: vec3f;
        var glow_col: vec3f;
        if (mt < 0.5) {
            interior = vec3f(0.02, 0.025, 0.04);
            edge = vec3f(0.04, 0.05, 0.08);
            glow_col = vec3f(0.03, 0.04, 0.07);
        } else if (mt < 1.5) {
            interior = vec3f(0.02, 0.04, 0.1);
            edge = vec3f(0.4, 0.7, 1.0);
            glow_col = vec3f(0.2, 0.45, 0.95);
        } else if (mt < 2.5) {
            interior = vec3f(0.1, 0.02, 0.02);
            edge = vec3f(1.0, 0.4, 0.3);
            glow_col = vec3f(0.95, 0.2, 0.15);
        } else if (mt < 3.5) {
            interior = vec3f(0.015, 0.03, 0.08);
            edge = vec3f(0.35, 0.6, 0.95);
            glow_col = vec3f(0.15, 0.35, 0.85);
        } else if (mt < 4.5) {
            interior = vec3f(0.08, 0.015, 0.01);
            edge = vec3f(0.95, 0.35, 0.25);
            glow_col = vec3f(0.85, 0.15, 0.1);
        } else {
            interior = vec3f(0.03);
            edge = vec3f(0.4, 0.45, 0.5);
            glow_col = vec3f(0.3, 0.35, 0.4);
        }

        col = mix(interior, edge, fresnel);

        let L1 = normalize(vec3f(0.4, 0.9, 0.3));
        let diff = max(dot(n, L1), 0.0) * 0.15;
        col += glow_col * diff;

        col += edge * fresnel * fresnel * 0.8;

        if (mt > 0.5 && mt < 2.5) {
            let nc2 = i32(P.neuron_count);
            for (var nn = 0; nn < nc2; nn++) {
                let ne = neurons[nn];
                if (ne.info.x < 0.01) {
                    continue;
                }
                let nc = ne.pos.xyz;
                let nd = length(p - nc);
                if (nd < 0.16) {
                    let pulse = 0.5 + 0.5 * sin(T * 2.2 + f32(nn) * 1.7);
                    let gc = select(vec3f(1.0, 0.4, 0.25), vec3f(0.4, 0.7, 1.0), ne.pos.w < 0.5);
                    col += gc * exp(-nd * nd * 400.0) * pulse * 0.5;
                    col += vec3f(0.95, 0.85, 0.7) * exp(-nd * nd * 3000.0) * 0.4 * pulse;
                }
            }
        }

        if (mt > 2.5 && mt < 4.5) {
            let cc2 = i32(P.conn_count);
            for (var c2 = 0; c2 < cc2; c2++) {
                let cn = conns[c2];
                if (cn.info.y < 0.01) {
                    continue;
                }
                let f2 = vec3f(cn.data.x, 0.05, cn.data.y);
                let tt = vec3f(cn.data.z, 0.05, cn.data.w);
                let ph = fract(T * 0.5 - hsh(vec2f(cn.data.x + cn.data.z, cn.data.y + cn.data.w)) * 10.0);
                var pp = mix(f2, tt, ph);
                pp.y = 0.055;
                let pd = length(p - pp);
                let pc = select(vec3f(1.0, 0.45, 0.3), vec3f(0.5, 0.75, 1.0), cn.info.x < 0.5);
                col += pc * exp(-pd * pd * 1500.0) * 0.7 * cn.info.y;
            }
        }

        col = mix(col, bg, 1.0 - exp(-t * 0.035));
    } else {
        let nc3 = i32(P.neuron_count);
        for (var nn = 0; nn < nc3; nn++) {
            let ne = neurons[nn];
            if (ne.info.x < 0.01) {
                continue;
            }
            let nc = ne.pos.xyz;
            let tc2 = max(0.0, dot(nc - ro, rd));
            let dd = length(ro + rd * tc2 - nc);
            let pulse = 0.5 + 0.5 * sin(T * 2.2 + f32(nn) * 1.7);
            let gc = select(vec3f(0.85, 0.15, 0.1), vec3f(0.15, 0.35, 0.85), ne.pos.w < 0.5);
            col += gc * exp(-dd * dd * 40.0) * pulse * 0.15;
            col += gc * 1.5 * exp(-dd * dd * 200.0) * pulse * 0.08;
        }
    }

    col = vec3f(1.0) - exp(-col * 2.0);
    col = pow(col, vec3f(0.42));

    let q = px / dims;
    col *= 0.5 + 0.5 * pow(16.0 * q.x * q.y * (1.0 - q.x) * (1.0 - q.y), 0.2);

    textureStore(out, vec2i(gid.xy), vec4f(col, 1.0));
}
