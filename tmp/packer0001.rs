use anyhow::{Context, Result, bail};
use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use si_format::audit::jit_audit;
pub use si_format::header::{
    SI_CANONICAL_MAGIC as SINT_PACKER_MAGIC, SI_CANONICAL_VERSION as MIN_VERSION,
};
pub use si_format::utils::{ALIGNMENT_BYTES, align_to_64, compute_padding};

/// Packer format version — v3 enforces tensor-descriptor manifest with explicit byte offsets
pub const SINT_PACKER_VERSION: u32 = 3;

/// Tuple representing an uncompressed tensor descriptor entry for container assembly:
/// (name, data_bytes, shape, is_lora, payload_type)
pub type RawTensorPayload = (String, Vec<u8>, Vec<usize>, bool, PayloadType);

// ────────────────────────────────────────────────────────────────────────────
// Tier Designation Flags (Offset 0x08 in .si SINT header)
// ────────────────────────────────────────────────────────────────────────────

pub use si_format::header::SiTierFlags;

// ────────────────────────────────────────────────────────────────────────────
// Manifest types
// ────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Partition {
    Model,     // Foundation Weights
    Expansion, // Capabilities & JIT Reflexes
    Domain,    // Context & Session Data
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PayloadType {
    Tensor,
    LoRA,
    JitReflex,
}

/// Describes a single tensor stored inside the `.si` container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorDescriptor {
    /// Human-readable tensor name (e.g. "lora_a", "ssm_in_proj")
    pub name: String,
    /// Logical shape, e.g. `[256, 16]` for an A-adapter matrix
    pub shape: Vec<usize>,
    /// Element dtype: "F32" (future: "F16", "BF16", "BYTECODE")
    pub dtype: String,
    /// Absolute byte offset from file start — always aligned to `ALIGNMENT_BYTES`
    pub byte_offset: u64,
    /// Total payload size in bytes
    pub byte_length: u64,
    /// `false` = immutable core SSM weight, `true` = mutable LoRA/adapter
    pub is_mutable: bool,
    /// Payload type (Tensor, LoRA, or JitReflex)
    #[serde(default = "default_payload_type")]
    pub payload_type: PayloadType,
}

fn default_payload_type() -> PayloadType {
    PayloadType::Tensor
}

/// Table-of-Contents manifest written after the file header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiContainerManifest {
    /// Unique model identifier / name
    pub model_identifier: String,
    /// Inner projection dimension (e.g. 256 or 4096)
    pub d_model: usize,
    /// SSM recurrent hidden state rank (e.g. 16 or 64)
    pub d_state: usize,
    /// LoRA adaptation rank r (e.g. 16 -> A: [d_model x r], B: [r x d_model])
    pub lora_rank: usize,
    /// Tier designation flags
    #[serde(default)]
    pub tier_flags: Option<SiTierFlags>,
    /// Ordered list of all tensor payloads in the container
    pub tensors: Vec<TensorDescriptor>,
}

// ────────────────────────────────────────────────────────────────────────────
// SiPacker — writes a .si container from raw f32 weight maps or bytecode
// ────────────────────────────────────────────────────────────────────────────

pub struct SiPacker;

impl SiPacker {
    /// Compiles a set of named f32 weight tensors (core SSM + dynamic LoRA) into a
    /// standards-compliant `.si` container at `output_path`.
    pub fn pack_to_si(
        output_path: &Path,
        model_id: &str,
        d_model: usize,
        d_state: usize,
        lora_rank: usize,
        core_weights: HashMap<String, Vec<f32>>,
    ) -> Result<()> {
        Self::pack_to_si_with_tier(
            output_path,
            model_id,
            SiTierFlags::TIER_3_REFLEX,
            d_model,
            d_state,
            lora_rank,
            core_weights,
        )
    }

    /// Compiles a set of named f32 weight tensors with explicit tier flags into a .si container.
    pub fn pack_to_si_with_tier(
        output_path: &Path,
        model_id: &str,
        tier: SiTierFlags,
        d_model: usize,
        d_state: usize,
        lora_rank: usize,
        core_weights: HashMap<String, Vec<f32>>,
    ) -> Result<()> {
        let mut all_tensors: Vec<RawTensorPayload> = Vec::new();

        let mut sorted_cores: Vec<(String, Vec<f32>)> = core_weights.into_iter().collect();
        sorted_cores.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, data) in sorted_cores {
            let shape = vec![data.len()];
            let bytes: Vec<u8> = data.iter().flat_map(|f| f.to_le_bytes()).collect();
            all_tensors.push((name, bytes, shape, false, PayloadType::Tensor));
        }

        // Dynamic LoRA adapter: A [d_model x lora_rank], B [lora_rank x d_model]
        all_tensors.push((
            "lora_a".to_string(),
            vec![0u8; d_model * lora_rank * 4],
            vec![d_model, lora_rank],
            true,
            PayloadType::LoRA,
        ));
        all_tensors.push((
            "lora_b".to_string(),
            vec![0u8; lora_rank * d_model * 4],
            vec![lora_rank, d_model],
            true,
            PayloadType::LoRA,
        ));

        let payload_sizes: Vec<u64> = all_tensors
            .iter()
            .map(|(_, data, _, _, _)| data.len() as u64)
            .collect();
        let header_prefix: u64 = 64; // Canonical SiCartridgeHeader is 64 bytes

        let compute_layout = |manifest_len_guess: u64| -> (Vec<u64>, u64) {
            let after_manifest = header_prefix + manifest_len_guess;
            let first_start = after_manifest + compute_padding(after_manifest) as u64;
            let mut offsets = Vec::with_capacity(payload_sizes.len());
            let mut cur = first_start;
            for &plen in &payload_sizes {
                cur += compute_padding(cur) as u64;
                offsets.push(cur);
                cur += plen;
                cur += compute_padding(cur) as u64;
            }
            (offsets, first_start)
        };

        let build_manifest = |offsets: &[u64]| -> SiContainerManifest {
            let descs: Vec<TensorDescriptor> = all_tensors
                .iter()
                .zip(&payload_sizes)
                .zip(offsets)
                .map(
                    |(((name, _, shape, mutable, ptype), &blen), &offset)| TensorDescriptor {
                        name: name.clone(),
                        shape: shape.clone(),
                        dtype: "F32".to_string(),
                        byte_offset: offset,
                        byte_length: blen,
                        is_mutable: *mutable,
                        payload_type: *ptype,
                    },
                )
                .collect();
            SiContainerManifest {
                model_identifier: model_id.to_string(),
                d_model,
                d_state,
                lora_rank,
                tier_flags: Some(tier),
                tensors: descs,
            }
        };

        let mut manifest_len_guess: u64 = 0;
        let mut manifest_bytes: Vec<u8>;
        let mut tensor_offsets: Vec<u64>;

        loop {
            let (offsets, _) = compute_layout(manifest_len_guess);
            tensor_offsets = offsets;
            let m = build_manifest(&tensor_offsets);
            manifest_bytes = bincode_serialize(&m)?;
            let new_len = manifest_bytes.len() as u64;
            if new_len == manifest_len_guess {
                break;
            }
            manifest_len_guess = new_len;
        }

        let manifest_len = manifest_len_guess;

        let header = si_format::header::SiCartridgeHeader {
            version: SINT_PACKER_VERSION as u16,
            flags: tier.bits(),
            block1_offset: 64,
            block1_len: manifest_len,
            ..Default::default()
        };

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(output_path)?;

        let header_bytes = header.to_bytes();
        file.write_all(&header_bytes)?;
        file.write_all(&manifest_bytes)?;

        let pad_after_manifest = compute_padding(64 + manifest_len);
        file.write_all(&vec![0u8; pad_after_manifest])?;

        let file_pos_after_header = 64 + manifest_len + pad_after_manifest as u64;
        let mut file_cursor = file_pos_after_header;

        for (((_, bytes, _, _, _), &expected_offset), &payload_len) in
            all_tensors.iter().zip(&tensor_offsets).zip(&payload_sizes)
        {
            let pad = compute_padding(file_cursor) as u64;
            if pad > 0 {
                file.write_all(&vec![0u8; pad as usize])?;
                file_cursor += pad;
            }

            debug_assert_eq!(
                file_cursor, expected_offset,
                "SiPacker: tensor offset mismatch: expected {}, at {}",
                expected_offset, file_cursor
            );

            file.write_all(bytes)?;
            file_cursor += payload_len;

            let trail = compute_padding(file_cursor) as u64;
            if trail > 0 {
                file.write_all(&vec![0u8; trail as usize])?;
                file_cursor += trail;
            }
        }

        file.flush()?;
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// SiSolidStateLoader — zero-copy memmap2 loader & tensor/reflex slicer
// ────────────────────────────────────────────────────────────────────────────

pub struct SiSolidStateLoader {
    pub manifest: SiContainerManifest,
    pub tier_flags: SiTierFlags,
    _file: File,
    mmap: Mmap,
}

impl SiSolidStateLoader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::load(path.as_ref())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        if mmap.len() < 64 {
            bail!("SiSolidStateLoader: {:?} too small for header", path);
        }

        let header: &si_format::header::SiCartridgeHeader = bytemuck::from_bytes(&mmap[0..64]);

        if header.magic != SINT_PACKER_MAGIC {
            bail!("SiSolidStateLoader: {:?} missing SINT magic bytes", path);
        }

        if header.version < MIN_VERSION {
            bail!(
                "Container version v{} is not supported (requires v{}+)",
                header.version,
                MIN_VERSION
            );
        }

        let tier_flags = SiTierFlags::from_bits(header.flags);

        let manifest_bytes = mmap
            .get(header.block1_offset as usize..(header.block1_offset + header.block1_len) as usize)
            .ok_or_else(|| anyhow::anyhow!("SiSolidStateLoader: TOC truncated"))?;

        let manifest: SiContainerManifest = bincode_deserialize(manifest_bytes)?;

        Ok(Self {
            manifest,
            tier_flags,
            _file: file,
            mmap,
        })
    }

    pub fn get_tensor_slice(&self, name: &str) -> Option<&[f32]> {
        let desc = self.manifest.tensors.iter().find(|t| t.name == name)?;
        let start = desc.byte_offset as usize;
        let end = start + desc.byte_length as usize;
        let raw_slice = self.mmap.get(start..end)?;

        debug_assert_eq!(
            raw_slice.as_ptr() as usize % ALIGNMENT_BYTES,
            0,
            "SiSolidStateLoader: tensor '{}' is not 64-byte aligned (ptr={:#x})",
            name,
            raw_slice.as_ptr() as usize
        );

        debug_assert_eq!(
            raw_slice.len() % 4,
            0,
            "Tensor byte length is not a multiple of 4"
        );
        let float_count = raw_slice.len() / 4;
        let f32_slice =
            unsafe { std::slice::from_raw_parts(raw_slice.as_ptr() as *const f32, float_count) };

        Some(f32_slice)
    }

    /// Loads a JIT Reflex and routes it through the Governance security audit gate.
    pub fn load_jit_reflex(&self, name: &str) -> Result<&[u8]> {
        let desc = self
            .manifest
            .tensors
            .iter()
            .find(|p| p.name == name && p.payload_type == PayloadType::JitReflex)
            .context(format!("JIT Reflex '{}' not found", name))?;

        let start = desc.byte_offset as usize;
        let end = start + desc.byte_length as usize;
        let bytecode = &self.mmap[start..end];

        // Governance security audit gate: prevents forbidden opcodes prior to PAGE_EXECUTE
        jit_audit(bytecode).context(format!("Governance JIT Audit FAILED for reflex: {}", name))?;

        Ok(bytecode)
    }

    pub fn is_mutable(&self, name: &str) -> bool {
        self.manifest
            .tensors
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.is_mutable)
            .unwrap_or(false)
    }

    pub fn shape(&self, name: &str) -> Option<&[usize]> {
        self.manifest
            .tensors
            .iter()
            .find(|t| t.name == name)
            .map(|t| t.shape.as_slice())
    }

    pub fn tensor_names(&self) -> Vec<&str> {
        self.manifest
            .tensors
            .iter()
            .map(|t| t.name.as_str())
            .collect()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Deterministic Serialization Helpers
// ────────────────────────────────────────────────────────────────────────────

fn bincode_serialize<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(bincode::serialize(value)?)
}

fn bincode_deserialize<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    Ok(bincode::deserialize(bytes)?)
}

#[cfg(test)]
#[allow(ambient_authority)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_padding_always_64_aligned() {
        for offset in 0u64..=256 {
            let pad = compute_padding(offset);
            assert!((offset as usize + pad).is_multiple_of(ALIGNMENT_BYTES));
        }
    }

    #[test]
    fn test_si_packer_roundtrip_and_alignment() {
        let temp_dir = tempfile::tempdir().expect("create test sandbox");
        let tmp = temp_dir.path().join("test_packer_roundtrip.si");
        let mut core = HashMap::new();
        core.insert("ssm_in_proj".to_string(), vec![0.1f32; 256 * 32]);
        core.insert("ssm_out_proj".to_string(), vec![0.2f32; 32 * 256]);

        SiPacker::pack_to_si(&tmp, "test_model", 32, 8, 4, core).expect("pack_to_si failed");

        let loader = SiSolidStateLoader::load(&tmp).expect("load failed");
        assert_eq!(loader.manifest.model_identifier, "test_model");
        assert_eq!(loader.manifest.d_model, 32);
        assert_eq!(loader.manifest.lora_rank, 4);

        let names = loader.tensor_names();
        assert!(names.contains(&"lora_a"));
        assert!(names.contains(&"lora_b"));
        assert!(names.contains(&"ssm_in_proj"));

        let in_proj = loader.get_tensor_slice("ssm_in_proj").unwrap();
        assert_eq!(in_proj.len(), 256 * 32);
        assert!((in_proj[0] - 0.1f32).abs() < 1e-6);
    }

    #[test]
    fn test_si_packer_writes_canonical_header() {
        let temp_dir = tempfile::tempdir().expect("create test sandbox");
        let tmp = temp_dir.path().join("test_packer_header.si");
        let mut core = HashMap::new();
        core.insert("ssm_in_proj".to_string(), vec![0.1f32; 256 * 32]);

        SiPacker::pack_to_si(&tmp, "test_model", 32, 8, 4, core).expect("pack_to_si failed");

        let file_bytes = std::fs::read(&tmp).expect("read si file");
        assert!(file_bytes.len() >= 64, "file too small for header");

        let header: &si_format::header::SiCartridgeHeader =
            bytemuck::from_bytes(&file_bytes[0..64]);

        assert_eq!(header.magic, si_format::header::SI_CANONICAL_MAGIC);
        assert_eq!(header.version, SINT_PACKER_VERSION as u16);
        assert_eq!(header.header_size, 64);
        assert_eq!(header.block1_offset, 64);
        assert_eq!(header.block1_len, file_bytes.len() as u64 - 64);
    }
}
