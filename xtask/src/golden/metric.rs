// Standard CIEDE2000 equations, unit parametric factors. ADR 0031.
pub(super) fn delta_e(a: [f64; 3], b: [f64; 3]) -> f64 {
    let c1 = a[1].hypot(a[2]);
    let c2 = b[1].hypot(b[2]);
    let cbar7 = ((c1 + c2) / 2.0).powi(7);
    let g = 0.5 * (1.0 - (cbar7 / (cbar7 + 25_f64.powi(7))).sqrt());
    let ap1 = (1.0 + g) * a[1];
    let ap2 = (1.0 + g) * b[1];
    let cp1 = ap1.hypot(a[2]);
    let cp2 = ap2.hypot(b[2]);
    let hue = |x: f64, y: f64| {
        if x == 0.0 && y == 0.0 {
            0.0
        } else {
            y.atan2(x).to_degrees().rem_euclid(360.0)
        }
    };
    let h1 = hue(ap1, a[2]);
    let h2 = hue(ap2, b[2]);
    let dl = b[0] - a[0];
    let dc = cp2 - cp1;
    let mut dh = h2 - h1;
    if cp1 * cp2 == 0.0 {
        dh = 0.0;
    } else if dh > 180.0 {
        dh -= 360.0;
    } else if dh < -180.0 {
        dh += 360.0;
    }
    let dh = 2.0 * (cp1 * cp2).sqrt() * (dh / 2.0).to_radians().sin();
    let lbar = (a[0] + b[0]) / 2.0;
    let cbar = (cp1 + cp2) / 2.0;
    let hbar = if cp1 * cp2 == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() <= 180.0 {
        (h1 + h2) / 2.0
    } else if h1 + h2 < 360.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        (h1 + h2 - 360.0) / 2.0
    };
    let cos = |v: f64| v.to_radians().cos();
    let t = 1.0 - 0.17 * cos(hbar - 30.0) + 0.24 * cos(2.0 * hbar) + 0.32 * cos(3.0 * hbar + 6.0)
        - 0.20 * cos(4.0 * hbar - 63.0);
    let sl = 1.0 + 0.015 * (lbar - 50.0).powi(2) / (20.0 + (lbar - 50.0).powi(2)).sqrt();
    let sc = 1.0 + 0.045 * cbar;
    let sh = 1.0 + 0.015 * cbar * t;
    let angle = 30.0 * (-((hbar - 275.0) / 25.0).powi(2)).exp();
    let rt = -2.0
        * (cbar.powi(7) / (cbar.powi(7) + 25_f64.powi(7))).sqrt()
        * (2.0 * angle).to_radians().sin();
    ((dl / sl).powi(2) + (dc / sc).powi(2) + (dh / sh).powi(2) + rt * (dc / sc) * (dh / sh))
        .max(0.0)
        .sqrt()
}

// PNG is straight-alpha sRGB; composite only for offline comparison, in linear light.
pub(super) fn lab(pixel: [u16; 4], background: f64) -> [f64; 3] {
    let alpha = f64::from(pixel[3]) / 65535.0;
    let rgb: [f64; 3] = std::array::from_fn(|i| {
        let s = f64::from(pixel[i]) / 65535.0;
        let linear = if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        };
        linear * alpha + background * (1.0 - alpha)
    });
    let xyz = [
        0.4124564 * rgb[0] + 0.3575761 * rgb[1] + 0.1804375 * rgb[2],
        0.2126729 * rgb[0] + 0.7151522 * rgb[1] + 0.0721750 * rgb[2],
        0.0193339 * rgb[0] + 0.1191920 * rgb[1] + 0.9503041 * rgb[2],
    ];
    let white = [0.95047, 1.0, 1.08883];
    let f: [f64; 3] = std::array::from_fn(|i| {
        let v = xyz[i] / white[i];
        if v > (6_f64 / 29.0).powi(3) {
            v.cbrt()
        } else {
            v / (3.0 * (6_f64 / 29.0).powi(2)) + 4.0 / 29.0
        }
    });
    [
        116.0 * f[1] - 16.0,
        500.0 * (f[0] - f[1]),
        200.0 * (f[1] - f[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sharma_reference_pairs_cover_hue_wrap_zero_chroma_and_dark_values() {
        // Numerical reference pairs from Sharma/Wu/Dalal's supplemental dataset.
        // https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/
        let pairs = [
            ([50., 2.6772, -79.7751], [50., 0., -82.7485], 2.0425),
            ([50., 3.1571, -77.2803], [50., 0., -82.7485], 2.8615),
            ([50., 0., 0.], [50., -1., 2.], 2.3669),
            ([50., 2.49, -0.001], [50., -2.49, 0.0009], 7.1792),
            ([50., 2.49, -0.001], [50., -2.49, 0.0011], 7.2195),
            ([50., -0.001, 2.49], [50., 0.0011, -2.49], 4.7461),
            ([50., 2.5, 0.], [73., 25., -18.], 27.1492),
            (
                [6.7747, -0.2908, -2.4247],
                [5.8714, -0.0985, -2.2286],
                0.6377,
            ),
            (
                [2.0776, 0.0795, -1.1350],
                [0.9033, -0.0636, -0.5514],
                0.9082,
            ),
        ];
        for (a, b, expected) in pairs {
            assert!((delta_e(a, b) - expected).abs() < 0.00005);
            assert!((delta_e(b, a) - expected).abs() < 0.00005);
            assert_eq!(delta_e(a, a), 0.0);
        }
    }
    #[test]
    fn lab_anchors_and_hidden_rgb() {
        assert_eq!(lab([0, 0, 0, 65535], 0.0), [0.0; 3]);
        let white = lab([65535; 4], 0.0);
        assert!(
            (white[0] - 100.0).abs() < 0.0001 && white[1].abs() < 0.0001 && white[2].abs() < 0.0001
        );
        assert_eq!(lab([65535, 0, 65535, 0], 1.0), lab([0; 4], 1.0));
    }
}
