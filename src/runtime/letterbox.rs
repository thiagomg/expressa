//! Keep a canvas's aspect ratio inside a free-size window (letterbox).

/// Destination rectangle for a `canvas_w`×`canvas_h` image inside `win_w`×`win_h`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Letterbox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn letterbox(win_w: f64, win_h: f64, canvas_w: f64, canvas_h: f64) -> Letterbox {
    let cw = canvas_w.max(1.0);
    let ch = canvas_h.max(1.0);
    let ww = win_w.max(1.0);
    let wh = win_h.max(1.0);
    let scale = (ww / cw).min(wh / ch);
    let width = (cw * scale).floor().max(1.0);
    let height = (ch * scale).floor().max(1.0);
    Letterbox {
        x: ((ww - width) * 0.5).floor(),
        y: ((wh - height) * 0.5).floor(),
        width,
        height,
    }
}

impl Letterbox {
    /// Canvas pixel under a window coordinate, or `None` on the bars.
    pub fn canvas_pixel(&self, x: f64, y: f64, canvas_w: f64, canvas_h: f64) -> Option<(i32, i32)> {
        if x < self.x || y < self.y || x >= self.x + self.width || y >= self.y + self.height {
            return None;
        }
        let cw = canvas_w.max(1.0);
        let ch = canvas_h.max(1.0);
        let px = ((x - self.x) / self.width * cw).floor() as i32;
        let py = ((y - self.y) / self.height * ch).floor() as i32;
        Some((px.clamp(0, cw as i32 - 1), py.clamp(0, ch as i32 - 1)))
    }

    pub fn as_i32(&self) -> (i32, i32, i32, i32) {
        (
            self.x as i32,
            self.y as i32,
            self.width as i32,
            self.height as i32,
        )
    }
}

/// Nearest-neighbour blit of `src` (`sw`×`sh`) into a `dw`×`dh` buffer,
/// centred, with `bar` filling the unused strips.
pub fn blit_letterbox(src: &[u32], sw: u32, sh: u32, dw: u32, dh: u32, bar: u32) -> Vec<u32> {
    let dw = dw.max(1);
    let dh = dh.max(1);
    let sw = sw.max(1);
    let sh = sh.max(1);
    let mut out = vec![bar; (dw * dh) as usize];
    let lb = letterbox(dw as f64, dh as f64, sw as f64, sh as f64);
    let (ox, oy, bw, bh) = lb.as_i32();
    let ox = ox.max(0) as u32;
    let oy = oy.max(0) as u32;
    let bw = (bw.max(1) as u32).min(dw.saturating_sub(ox));
    let bh = (bh.max(1) as u32).min(dh.saturating_sub(oy));
    for y in 0..bh {
        let sy = y * sh / bh;
        let dst_row = ((oy + y) * dw + ox) as usize;
        let src_row = (sy * sw) as usize;
        for x in 0..bw {
            let sx = (x * sw / bw) as usize;
            if let Some(&px) = src.get(src_row + sx)
                && let Some(slot) = out.get_mut(dst_row + x as usize)
            {
                *slot = px;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_window_fills_the_canvas() {
        let lb = letterbox(320.0, 200.0, 320.0, 200.0);
        assert_eq!(
            lb,
            Letterbox {
                x: 0.0,
                y: 0.0,
                width: 320.0,
                height: 200.0
            }
        );
        assert_eq!(lb.canvas_pixel(0.0, 0.0, 320.0, 200.0), Some((0, 0)));
        assert_eq!(
            lb.canvas_pixel(319.0, 199.0, 320.0, 200.0),
            Some((319, 199))
        );
    }

    #[test]
    fn wide_window_adds_side_bars() {
        let lb = letterbox(400.0, 200.0, 320.0, 200.0);
        assert_eq!(lb.height, 200.0);
        assert_eq!(lb.width, 320.0);
        assert_eq!(lb.x, 40.0);
        assert_eq!(lb.y, 0.0);
        assert_eq!(lb.canvas_pixel(39.0, 100.0, 320.0, 200.0), None);
        assert_eq!(lb.canvas_pixel(40.0, 0.0, 320.0, 200.0), Some((0, 0)));
        assert_eq!(
            lb.canvas_pixel(200.0, 100.0, 320.0, 200.0),
            Some((160, 100))
        );
        assert_eq!(lb.canvas_pixel(360.0, 100.0, 320.0, 200.0), None);
    }

    #[test]
    fn tall_window_adds_top_and_bottom_bars() {
        let lb = letterbox(320.0, 400.0, 320.0, 200.0);
        assert_eq!(lb.width, 320.0);
        assert_eq!(lb.height, 200.0);
        assert_eq!(lb.x, 0.0);
        assert_eq!(lb.y, 100.0);
        assert_eq!(lb.canvas_pixel(10.0, 99.0, 320.0, 200.0), None);
        assert_eq!(lb.canvas_pixel(10.0, 100.0, 320.0, 200.0), Some((10, 0)));
    }

    #[test]
    fn blit_keeps_aspect_and_fills_bars() {
        let src = vec![1u32, 2, 3, 4];
        let out = blit_letterbox(&src, 2, 2, 4, 2, 9);
        // scale 1, dest 2×2 centred in 4×2 → x=1
        assert_eq!(out, vec![9, 1, 2, 9, 9, 3, 4, 9]);
    }
}
