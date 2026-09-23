//! Minimal `.npy` v1.0 writer for the sharded dataset format.
//!
//! The training pipeline memory-maps these files with
//! `np.memmap(path, mode='r')` — so the writer must emit exactly the
//! canonical v1.0 framing: six-byte magic, version bytes, a little-endian
//! `u16` header length, a Python-dict header padded so the payload starts on
//! a 64-byte boundary, then row-major little-endian element bytes. There is
//! deliberately no reader here; the tests parse the header back to pin the
//! contract.
//!
//! Ported from a sibling research codebase's `learn/npy.rs` verbatim — the container
//! contract is game-independent.

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

/// Element dtype tags understood by the writer.
#[derive(Debug, Clone, Copy)]
pub enum DType {
    /// `<f4` — little-endian IEEE float32.
    F32,
    /// `|b1` — NumPy boolean (one byte per element).
    Bool,
    /// `<u8` — little-endian uint64.
    U64,
}

impl DType {
    fn descr(self) -> &'static str {
        match self {
            Self::F32 => "<f4",
            Self::Bool => "|b1",
            Self::U64 => "<u8",
        }
    }
}

fn shape_literal(shape: &[usize]) -> String {
    if shape.len() == 1 {
        format!("({},)", shape[0])
    } else {
        let inner: Vec<String> = shape.iter().map(|d| d.to_string()).collect();
        format!("({})", inner.join(", "))
    }
}

fn write_header(file: &mut impl Write, dtype: DType, shape: &[usize]) -> std::io::Result<()> {
    let dict = format!(
        "{{'descr': '{}', 'fortran_order': False, 'shape': {}, }}",
        dtype.descr(),
        shape_literal(shape)
    );
    // v1.0: magic (6) + version (2) + header-len (2) + header, padded so the
    // whole preamble lands on a 64-byte boundary and ends with '\n'.
    let preamble = 10;
    let padded_len = {
        let raw = preamble + dict.len() + 1;
        let rem = raw % 64;
        let pad = if rem == 0 { 0 } else { 64 - rem };
        dict.len() + pad + 1
    };
    let mut header = dict;
    let pad = padded_len - header.len() - 1;
    header.push_str(&" ".repeat(pad));
    header.push('\n');
    debug_assert_eq!((preamble + header.len()) % 64, 0);

    file.write_all(b"\x93NUMPY")?;
    file.write_all(&[0x01, 0x00])?;
    file.write_all(&(header.len() as u16).to_le_bytes())?;
    file.write_all(header.as_bytes())
}

/// Writes `data` (row-major, C order) as a `.npy` tensor of `shape`.
pub fn write_f32(path: &Path, shape: &[usize], data: &[f32]) -> std::io::Result<()> {
    debug_assert_eq!(shape.iter().product::<usize>(), data.len());
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, DType::F32, shape)?;
    // One buffer + one write: per-element `write_all` would issue a syscall
    // per scalar, dominating dataset generation for multi-million-row shards.
    let bytes: Vec<u8> = data.iter().flat_map(|value| value.to_le_bytes()).collect();
    file.write_all(&bytes)?;
    file.flush()
}

/// Writes a boolean tensor (one `|b1` byte per element).
pub fn write_bool(path: &Path, shape: &[usize], data: &[bool]) -> std::io::Result<()> {
    debug_assert_eq!(shape.iter().product::<usize>(), data.len());
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, DType::Bool, shape)?;
    let bytes: Vec<u8> = data.iter().map(|value| *value as u8).collect();
    file.write_all(&bytes)?;
    file.flush()
}

/// Writes a little-endian `u64` tensor (used for provenance keys).
pub fn write_u64(path: &Path, shape: &[usize], data: &[u64]) -> std::io::Result<()> {
    debug_assert_eq!(shape.iter().product::<usize>(), data.len());
    let mut file = BufWriter::new(File::create(path)?);
    write_header(&mut file, DType::U64, shape)?;
    let bytes: Vec<u8> = data.iter().flat_map(|value| value.to_le_bytes()).collect();
    file.write_all(&bytes)?;
    file.flush()
}

/// FNV-1a 64-bit digest over a file's bytes — the repository's established
/// provenance convention (`fnv1a64:<hex>`), shared with benchmark artifacts.
pub fn fnv1a64_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(fnv1a64_bytes(&bytes))
}

pub fn fnv1a64_bytes(bytes: &[u8]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn read_header(path: &Path) -> (String, Vec<u8>) {
        let mut file = File::open(path).unwrap();
        let mut magic = [0_u8; 6];
        file.read_exact(&mut magic).unwrap();
        assert_eq!(&magic, b"\x93NUMPY");
        let mut version = [0_u8; 2];
        file.read_exact(&mut version).unwrap();
        assert_eq!(version, [1, 0]);
        let mut len = [0_u8; 2];
        file.read_exact(&mut len).unwrap();
        let len = u16::from_le_bytes(len) as usize;
        let mut header = vec![0_u8; len];
        file.read_exact(&mut header).unwrap();
        let mut payload = Vec::new();
        file.read_to_end(&mut payload).unwrap();
        (String::from_utf8(header).unwrap(), payload)
    }

    #[test]
    fn the_header_is_a_parseable_aligned_preamble() {
        let dir = std::env::temp_dir().join("koi_npy_test_header");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("features.npy");
        write_f32(&path, &[2, 4], &[1.0, 2.5, -3.0, 4.25, 0.0, 1.0, 2.0, 3.0]).unwrap();
        let (header, payload) = read_header(&path);
        assert!(header.contains("'descr': '<f4'"));
        assert!(header.contains("'fortran_order': False"));
        assert!(header.contains("'shape': (2, 4)"));
        assert!(header.ends_with('\n'));
        assert_eq!((10 + header.len()) % 64, 0, "payload must be 64-byte aligned");
        assert_eq!(payload.len(), 8 * 4);
        let first = f32::from_le_bytes(payload[0..4].try_into().unwrap());
        assert_eq!(first, 1.0);
        let second = f32::from_le_bytes(payload[4..8].try_into().unwrap());
        assert_eq!(second, 2.5);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn one_dimensional_shapes_use_the_singleton_literal() {
        let dir = std::env::temp_dir().join("koi_npy_test_1d");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ev.npy");
        write_f32(&path, &[3], &[0.5, -1.0, 2.0]).unwrap();
        let (header, _) = read_header(&path);
        assert!(header.contains("'shape': (3,)"), "header: {header}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bool_and_u64_tensors_round_trip() {
        let dir = std::env::temp_dir().join("koi_npy_test_misc");
        std::fs::create_dir_all(&dir).unwrap();
        let mask = dir.join("mask.npy");
        write_bool(&mask, &[2, 2], &[true, false, false, true]).unwrap();
        let (header, payload) = read_header(&mask);
        assert!(header.contains("'descr': '|b1'"));
        assert_eq!(payload, vec![1, 0, 0, 1]);

        let prov = dir.join("prov.npy");
        write_u64(&prov, &[2], &[u64::MAX, 7]).unwrap();
        let (header, payload) = read_header(&prov);
        assert!(header.contains("'descr': '<u8'"));
        assert_eq!(payload.len(), 16);
        assert_eq!(u64::from_le_bytes(payload[8..16].try_into().unwrap()), 7);
        std::fs::remove_dir_all(&dir).ok();
    }
}
