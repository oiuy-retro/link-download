use std::fs;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let icon = out.join("app.ico");
    fs::write(&icon, make_ico()).unwrap();
    let manifest = out.join("app.manifest");
    fs::write(&manifest, MANIFEST).unwrap();

    let mut res = winres::WindowsResource::new();
    res.set_icon(icon.to_str().unwrap());
    res.set_manifest_file(manifest.to_str().unwrap());
    res.set("FileDescription", "Link Download");
    res.set("ProductName", "Link Download");
    res.set("OriginalFilename", "Link Download.exe");
    res.compile().expect("embed the icon and manifest");
    // GNU ld drops the icon unless the resource archive is pulled in whole.
    let lib = out.join("libresource.a");
    println!("cargo:rustc-link-arg-bins=-Wl,--whole-archive");
    println!("cargo:rustc-link-arg-bins={}", lib.display());
    println!("cargo:rustc-link-arg-bins=-Wl,--no-whole-archive");
}

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity version="1.0.1.0" name="Link.Download" type="win32"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true</dpiAware>
    </windowsSettings>
  </application>
</assembly>
"#;

fn make_ico() -> Vec<u8> {
    let images = [(16u32, paint(16)), (32, paint(32)), (48, paint(48)), (256, paint(256))];
    let mut file = Vec::new();
    file.extend_from_slice(&0u16.to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + images.len() * 16;
    let mut blobs = Vec::new();
    for (size, pixels) in &images {
        let size = *size;
        let xor = size * size * 4;
        let mask_stride = ((size + 31) / 32) * 4;
        let mask = mask_stride * size;
        let bytes = 40 + xor + mask;
        file.push(if size == 256 { 0 } else { size as u8 });
        file.push(if size == 256 { 0 } else { size as u8 });
        file.push(0);
        file.push(0);
        file.extend_from_slice(&1u16.to_le_bytes());
        file.extend_from_slice(&32u16.to_le_bytes());
        file.extend_from_slice(&(bytes as u32).to_le_bytes());
        file.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += bytes as usize;
        blobs.push(dib(pixels, size));
    }
    for blob in blobs {
        file.extend_from_slice(&blob);
    }
    file
}

fn dib(pixels: &[u8], size: u32) -> Vec<u8> {
    let mut blob = Vec::new();
    blob.extend_from_slice(&40u32.to_le_bytes());
    blob.extend_from_slice(&(size as i32).to_le_bytes());
    blob.extend_from_slice(&((size as i32) * 2).to_le_bytes());
    blob.extend_from_slice(&1u16.to_le_bytes());
    blob.extend_from_slice(&32u16.to_le_bytes());
    blob.extend_from_slice(&0u32.to_le_bytes());
    blob.extend_from_slice(&0u32.to_le_bytes());
    blob.extend_from_slice(&0i32.to_le_bytes());
    blob.extend_from_slice(&0i32.to_le_bytes());
    blob.extend_from_slice(&0u32.to_le_bytes());
    blob.extend_from_slice(&0u32.to_le_bytes());
    let stride = size as usize;
    for y in (0..stride).rev() {
        let start = y * stride * 4;
        blob.extend_from_slice(&pixels[start..start + stride * 4]);
    }
    let mask_stride = ((size + 31) / 32) * 4;
    for _ in 0..size {
        blob.extend(std::iter::repeat(0).take(mask_stride as usize));
    }
    blob
}

fn paint(size: usize) -> Vec<u8> {
    let mut px = vec![0u8; size * size * 4];
    let mut put = |x: i32, y: i32, r: u8, g: u8, b: u8| {
        if x < 0 || y < 0 || x >= size as i32 || y >= size as i32 {
            return;
        }
        let i = (y as usize * size + x as usize) * 4;
        px[i] = b;
        px[i + 1] = g;
        px[i + 2] = r;
        px[i + 3] = 255;
    };
    let s = size as i32;
    let margin = (s / 16).max(1);
    for y in 0..s {
        for x in 0..s {
            put(x, y, 192, 192, 192);
        }
    }
    let left = margin;
    let top = margin;
    let right = s - margin;
    let bottom = s - margin;
    for y in top..bottom {
        for x in left..right {
            put(x, y, 255, 255, 255);
        }
    }
    for x in left..right {
        put(x, top, 0, 0, 128);
        put(x, bottom - 1, 0, 0, 128);
    }
    for y in top..bottom {
        put(left, y, 0, 0, 128);
        put(right - 1, y, 0, 0, 128);
    }
    let sun = (s / 6).max(2);
    let sun_x = left + sun + margin;
    let sun_y = top + sun + margin;
    for y in (sun_y - sun / 2)..(sun_y + sun / 2) {
        for x in (sun_x - sun / 2)..(sun_x + sun / 2) {
            let dx = x - sun_x;
            let dy = y - sun_y;
            if dx * dx + dy * dy <= (sun / 2) * (sun / 2) {
                put(x, y, 255, 200, 0);
            }
        }
    }
    let ground = bottom - margin - 1;
    for y in (top + (bottom - top) / 2)..ground {
        let rise = ground - y;
        for x in left + 1..right - 1 {
            let hill = ((x - left) * 2 / (right - left).max(1)) - 1;
            let height = (s / 5) - hill.abs() * (s / 10);
            if rise < height.max(2) {
                put(x, y, 0, 128, 70);
            }
        }
    }
    let badge = (s / 3).max(5);
    let bx = right - badge - 1;
    let by = bottom - badge - 1;
    for y in by..(by + badge) {
        for x in bx..(bx + badge) {
            put(x, y, 0, 0, 128);
        }
    }
    let shaft_x = bx + badge / 2;
    let shaft_top = by + 1;
    let shaft_bot = by + badge - (badge / 3).max(2);
    for y in shaft_top..shaft_bot {
        put(shaft_x, y, 255, 255, 255);
        if badge > 8 {
            put(shaft_x - 1, y, 255, 255, 255);
        }
    }
    let head = (badge / 3).max(2);
    for i in 0..head {
        let y = shaft_bot + i;
        for x in (shaft_x - head + i)..(shaft_x + head - i + 1) {
            put(x, y, 255, 255, 255);
        }
    }
    px
}
