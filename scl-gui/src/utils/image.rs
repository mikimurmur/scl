//! 图片编解码辅助函数。

use anyhow::Context;
use image::DynamicImage;

/// 把一个 [`DynamicImage`] 编码为紧凑的字符串
///
/// 编码格式为 `ascii85(宽 u32 大端 | 高 u32 大端 | RGBA 像素)`。
pub fn from_image_to_string(image: &DynamicImage) -> anyhow::Result<String> {
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let raw = rgba.into_raw();
    let mut buffer = Vec::with_capacity(8 + raw.len());
    write_u32(&mut buffer, width);
    write_u32(&mut buffer, height);
    buffer.extend_from_slice(&raw);
    Ok(ascii85_encode(&buffer))
}

/// 从字节切片的指定偏移读取一个大端序 `u32`
///
/// 与官方实现一致：偏移越界会直接 panic。
pub fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// 在字节缓冲区末尾追加一个大端序 `u32`
pub fn write_u32(buffer: &mut Vec<u8>, value: u32) {
    buffer.extend_from_slice(&value.to_be_bytes());
}

/// 解码官方格式的图标字符串，返回宽、高和 RGBA 像素
pub fn decode_image_buf(encoded: &str) -> anyhow::Result<(usize, usize, Vec<u8>)> {
    let bytes = ascii85_decode(encoded).context("解码模组图标失败")?;
    anyhow::ensure!(bytes.len() > 7, "模组图标数据不足");
    let width = read_u32(&bytes, 0) as usize;
    let height = read_u32(&bytes, 4) as usize;
    anyhow::ensure!(width > 0 && height > 0, "模组图标尺寸无效");
    let expected = width
        .checked_mul(height)
        .and_then(|value| value.checked_mul(4))
        .context("模组图标尺寸溢出")?;
    anyhow::ensure!(bytes.len() >= 8 + expected, "模组图标像素数据长度不足");
    Ok((width, height, bytes[8..8 + expected].to_vec()))
}

/// 标准 ASCII85 编码，输出带 `<~` / `~>` 定界符
fn ascii85_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 5 / 4 + 4);
    output.push_str("<~");
    for chunk in bytes.chunks(4) {
        let mut group = [0u8; 4];
        group[..chunk.len()].copy_from_slice(chunk);
        let value = u32::from_be_bytes(group);
        let mut digits = [0u8; 5];
        let mut rest = value;
        for digit in digits.iter_mut().rev() {
            *digit = (rest % 85) as u8;
            rest /= 85;
        }
        let count = if chunk.len() == 4 { 5 } else { chunk.len() + 1 };
        for &digit in &digits[..count] {
            output.push((b'!' + digit) as char);
        }
    }
    output.push_str("~>");
    output
}

/// 标准 ASCII85 解码，兼容 `<~` / `~>` 定界符与 `z` 缩写
fn ascii85_decode(input: &str) -> anyhow::Result<Vec<u8>> {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix("<~").unwrap_or(trimmed);
    let body = body.strip_suffix("~>").unwrap_or(body);
    let mut output = Vec::new();
    let mut group = [0u8; 5];
    let mut count = 0;
    for ch in body.chars() {
        if ch.is_ascii_whitespace() {
            continue;
        }
        if ch == 'z' {
            anyhow::ensure!(count == 0, "Missaligned z in input");
            output.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let digit = (ch as u32).wrapping_sub('!' as u32);
        anyhow::ensure!(digit <= 84, "Input char is out of range for Ascii85");
        group[count] = digit as u8;
        count += 1;
        if count == 5 {
            let value = group
                .iter()
                .fold(0u32, |value, &digit| value * 85 + digit as u32);
            output.extend_from_slice(&value.to_be_bytes());
            count = 0;
        }
    }
    anyhow::ensure!(count != 1, "ASCII85 数据以单个字符结尾");
    if count > 1 {
        for digit in group.iter_mut().skip(count) {
            *digit = 84;
        }
        let value = group
            .iter()
            .fold(0u32, |value, &digit| value * 85 + digit as u32);
        let decoded = value.to_be_bytes();
        output.extend_from_slice(&decoded[..count - 1]);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii85_known_vectors() {
        assert_eq!(ascii85_encode(b""), "<~~>");
        assert_eq!(ascii85_encode(b"Hello world!"), "<~87cURD]j7BEbo80~>");
        assert_eq!(ascii85_encode(b"1234"), "<~0etOA~>");
        assert_eq!(ascii85_encode(b"123"), "<~0etN~>");
    }

    #[test]
    fn ascii85_round_trip() {
        for len in 0..=8 {
            let data = (0..len)
                .map(|x| (x as u8).wrapping_mul(37))
                .collect::<Vec<_>>();
            let encoded = ascii85_encode(&data);
            assert!(encoded.starts_with("<~"));
            assert!(encoded.ends_with("~>"));
            assert_eq!(ascii85_decode(&encoded).unwrap(), data);
        }
    }

    #[test]
    fn image_string_round_trip() {
        let image = DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            2,
            2,
            image::Rgba([1, 2, 3, 4]),
        ));
        let encoded = from_image_to_string(&image).unwrap();
        let (width, height, pixels) = decode_image_buf(&encoded).unwrap();
        assert_eq!((width, height), (2, 2));
        assert_eq!(pixels.len(), 16);
        assert_eq!(&pixels[..4], &[1, 2, 3, 4]);
    }
}
