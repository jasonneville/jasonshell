#[cfg(test)]
mod memory_acceptance {
    use super::*;
    pub(super) fn image(w: u32, h: u32) -> CroppedImage {
        let mut p = vec![0; w as usize * h as usize * 4];
        for (i, p) in p.chunks_exact_mut(4).enumerate() {
            p.copy_from_slice(&[(i % 251) as u8, (i % 241) as u8, (i % 239) as u8, 255]);
        }
        CroppedImage {
            width: w,
            height: h,
            pixels: p,
        }
    }
    fn bytes(g: &Global) -> Vec<u8> {
        unsafe {
            let p = GlobalLock(g.handle);
            assert!(!p.is_null());
            let v = std::slice::from_raw_parts(p.cast::<u8>(), GlobalSize(g.handle)).to_vec();
            let _ = GlobalUnlock(g.handle);
            v
        }
    }
    fn exercise(w: u32, h: u32) -> (usize, usize) {
        let i = image(w, h);
        let initial = owned_counts();
        assert_eq!(
            (initial.live_global_allocations, initial.live_staged_bytes),
            (0, 0)
        );
        TEST_PEAK.store(0, Ordering::SeqCst);
        let t = Instant::now();
        let p = prepare(&i, i.pixels.len(), Failpoints::default()).unwrap();
        let elapsed = t.elapsed();
        let peak = TEST_PEAK.load(Ordering::SeqCst);
        let d = bytes(&p.dib);
        let n = bytes(&p.png);
        assert_eq!(p.dib.logical_len, i.pixels.len() + 40);
        assert_eq!(&d[..4], &40u32.to_le_bytes());
        assert_eq!(&d[4..8], &(w as i32).to_le_bytes());
        assert_eq!(&d[8..12], &(h as i32).to_le_bytes());
        assert_eq!(&d[12..16], &[1, 0, 32, 0]);
        assert_eq!(&d[20..24], &(i.pixels.len() as u32).to_le_bytes());
        for (row, src) in d[40..p.dib.logical_len]
            .chunks_exact(w as usize * 4)
            .zip(i.pixels.chunks_exact(w as usize * 4).rev())
        {
            for (a, b) in row.chunks_exact(4).zip(src.chunks_exact(4)) {
                assert_eq!(a, [b[2], b[1], b[0], 255]);
            }
        }
        assert!(d[p.dib.logical_len..].iter().all(|b| *b == 0));
        let mut expected = Vec::with_capacity(png_lengths(&i).unwrap().2);
        encode_png(&i, &mut expected).unwrap();
        assert_eq!(&n[..p.png.logical_len], &expected);
        assert!(n[p.png.logical_len..].iter().all(|b| *b == 0));
        let release_bound = d.len() + n.len() + expected.capacity();
        println!("clipboard_actual_prepare {w}x{h} staged_peak={peak} release_bound={release_bound} dib_logical={} dib_GlobalSize={} png_logical={} png_GlobalSize={} prepare_ns={}",p.dib.logical_len,d.len(),p.png.logical_len,n.len(),elapsed.as_nanos());
        drop(p);
        assert_eq!(owned_counts().live_global_allocations, 0);
        assert_eq!(owned_counts().live_staged_bytes, 0);
        assert_eq!(BYTES.load(Ordering::SeqCst), 0);
        (peak, release_bound)
    }
    #[test]
    fn actual_prepare_before() {
        for (w, h) in [(1, 1), (3, 7), (1920, 1080), (3840, 2160)] {
            exercise(w, h);
        }
    }
    #[test]
    fn acceptance_actual_prepare_releases_dib() {
        for (w, h) in [(1, 1), (3, 7), (1920, 1080), (3840, 2160)] {
            let (p, b) = exercise(w, h);
            assert!(
                p <= b,
                "DIB Vec and Staged must release before PNG global allocation"
            );
        }
    }
    #[test]
    fn actual_global_rounding_failure_cleanup() {
        for n in [1, 3, 7, 15, 17, 65535] {
            let input = vec![37; n];
            let g = Global::copy(&input, false).unwrap();
            let b = bytes(&g);
            assert_eq!(&b[..n], &input);
            assert!(b[n..].iter().all(|b| *b == 0));
            assert_eq!(g.logical_len, n);
            drop(g);
            assert_eq!(owned_counts().live_staged_bytes, 0);
            assert_eq!(owned_counts().live_global_allocations, 0);
        }
        assert!(matches!(Global::copy(&[], false), Err(Failure::Allocation)));
        assert!(matches!(Global::copy(&[1], true), Err(Failure::Allocation)));
        let i = image(1, 1);
        assert!(matches!(
            prepare(
                &i,
                4,
                Failpoints {
                    allocation: true,
                    ..Default::default()
                }
            ),
            Err(Failure::Allocation)
        ));
        assert_eq!(BYTES.load(Ordering::SeqCst), 0);
        assert_eq!(owned_counts().live_staged_bytes, 0);
        assert_eq!(owned_counts().live_global_allocations, 0);
    }
    #[test]
    fn budget_png_copy_failure_cleans_dib() {
        let i = image(3, 7);
        let len = i.pixels.len();
        let png = png_lengths(&i).unwrap().2;
        let conservative = len + (len + 40) + png;
        let blocker = Charge::new(MAX_STAGING_BYTES - conservative - (len + 40) - png + 1).unwrap();
        assert!(matches!(
            prepare(&i, len, Failpoints::default()),
            Err(Failure::Budget)
        ));
        assert_eq!(owned_counts().live_global_allocations, 0);
        assert_eq!(owned_counts().live_staged_bytes, 0);
        drop(blocker);
        assert_eq!(BYTES.load(Ordering::SeqCst), 0);
        assert!(matches!(
            prepare(&i, MAX_STAGING_BYTES, Failpoints::default()),
            Err(Failure::Budget)
        ));
        assert_eq!(BYTES.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn transfer_local_ownership() {
        let g = Global::copy(&[3, 2, 1], false).unwrap();
        let h = g.transfer();
        assert_eq!(owned_counts().live_global_allocations, 0);
        assert_eq!(owned_counts().live_staged_bytes, 0);
        assert_eq!(BYTES.load(Ordering::SeqCst), 0);
        unsafe {
            assert_eq!(GlobalSize(h), 3);
            let _ = GlobalFree(h);
        }
    }
}
const fn crc_table_data() -> [u32; 256] {
    let mut t = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut v = i as u32;
        let mut j = 0;
        while j < 8 {
            v = (v >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(v & 1));
            j += 1
        }
        t[i] = v;
        i += 1
    }
    t
}
static CRC_TABLE: [u32; 256] = crc_table_data();
fn crc_table(b: &[u8]) -> u32 {
    let mut v = u32::MAX;
    for b in b {
        v = (v >> 8) ^ CRC_TABLE[((v ^ u32::from(*b)) & 255) as usize]
    }
    !v
}
#[test]
fn complete_png_crc_table_equivalence() {
    for (w, h) in [(1, 1), (3, 7), (257, 65), (1920, 1080), (3840, 2160)] {
        let i = memory_acceptance::image(w, h);
        let cap = png_lengths(&i).unwrap().2;
        let mut a = Vec::with_capacity(cap);
        let mut b = Vec::with_capacity(cap);
        encode_rgba(w, h, &i.pixels, &mut a).unwrap();
        encode_rgba_table(w, h, &i.pixels, &mut b).unwrap();
        assert_eq!(a, b, "{w}x{h}");
    }
}
