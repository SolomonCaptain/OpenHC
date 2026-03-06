//! NPU 类型系统
//!
//! 定义 NPU 统一类型及其与 AST 类型的映射关系。
//! 支持 NPU 特有的量化类型和张量布局。

use std::fmt;

/// NPU 类型种类
#[derive(Debug, Clone, PartialEq)]
pub enum NpuTypeKind {
    /// 整数类型
    Integer { width: u32, signed: bool },
    /// 浮点类型
    Float { width: u32 },
    /// 量化类型（NPU 特有）
    Quantized {
        base: QuantBase,
        scale: f32,
        zero_point: i32,
    },
    /// 张量类型
    Tensor {
        element: Box<NpuTypeKind>,
        shape: Vec<i64>,
        layout: TensorLayout,
    },
    /// 张量切片（NPU 优化关键）
    TensorSlice {
        source: Box<NpuTypeKind>,
        offsets: Vec<i64>,
        sizes: Vec<i64>,
    },
    /// Void 类型
    Void,
}

/// 量化基础类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantBase {
    /// INT8 (最常用)
    Int8,
    /// UINT8
    UInt8,
    /// INT4 (低精度推理)
    Int4,
    /// UINT4
    UInt4,
    /// FP8 (E4M3/E5M2)
    FP8,
    /// BF16 (Brain Float)
    BF16,
}

/// 张量布局（影响 NPU 性能关键）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorLayout {
    /// 行主序 (C-style)
    RowMajor,
    /// 列主序 (Fortran-style)
    ColMajor,
    /// NCHW (Batch, Channel, Height, Width)
    NCHW,
    /// NHWC (NPU 通常更友好)
    NHWC,
    /// NCxHxW (块化布局，昇腾专用)
    NCHWc { c_block: u32 },
    /// 压缩稀疏格式
    Compressed { format: SparseFormat },
    /// 未知/默认
    Unknown,
}

impl Default for TensorLayout {
    fn default() -> Self {
        TensorLayout::NCHW
    }
}

/// 稀疏格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparseFormat {
    /// CSR (Compressed Sparse Row)
    CSR,
    /// CSC (Compressed Sparse Column)
    CSC,
    /// COO (Coordinate Format)
    COO,
    /// Block Sparse
    BlockSparse { block_size: u32 },
}

/// NPU 类型
#[derive(Debug, Clone)]
pub struct NpuType {
    /// 类型种类
    pub kind: NpuTypeKind,
    /// 设备提示（可选）
    pub device_hint: Option<String>,
}

impl NpuType {
    /// 创建整数类型
    pub fn integer(width: u32, signed: bool) -> Self {
        Self {
            kind: NpuTypeKind::Integer { width, signed },
            device_hint: None,
        }
    }

    /// 创建 i8 类型
    pub fn i8() -> Self {
        Self::integer(8, true)
    }

    /// 创建 i16 类型
    pub fn i16() -> Self {
        Self::integer(16, true)
    }

    /// 创建 i32 类型
    pub fn i32() -> Self {
        Self::integer(32, true)
    }

    /// 创建 i64 类型
    pub fn i64() -> Self {
        Self::integer(64, true)
    }

    /// 创建 u8 类型
    pub fn u8() -> Self {
        Self::integer(8, false)
    }

    /// 创建 u32 类型
    pub fn u32() -> Self {
        Self::integer(32, false)
    }

    /// 创建浮点类型
    pub fn float(width: u32) -> Self {
        Self {
            kind: NpuTypeKind::Float { width },
            device_hint: None,
        }
    }

    /// 创建 float16 类型
    pub fn fp16() -> Self {
        Self::float(16)
    }

    /// 创建 float32 类型
    pub fn f32() -> Self {
        Self::float(32)
    }

    /// 创建 float64 类型
    pub fn f64() -> Self {
        Self::float(64)
    }

    /// 创建 bf16 类型
    pub fn bf16() -> Self {
        Self {
            kind: NpuTypeKind::Quantized {
                base: QuantBase::BF16,
                scale: 1.0,
                zero_point: 0,
            },
            device_hint: None,
        }
    }

    /// 创建量化 INT8 类型
    pub fn quant_int8(scale: f32, zero_point: i32) -> Self {
        Self {
            kind: NpuTypeKind::Quantized {
                base: QuantBase::Int8,
                scale,
                zero_point,
            },
            device_hint: None,
        }
    }

    /// 创建量化 FP8 类型
    pub fn quant_fp8(scale: f32, zero_point: i32) -> Self {
        Self {
            kind: NpuTypeKind::Quantized {
                base: QuantBase::FP8,
                scale,
                zero_point,
            },
            device_hint: None,
        }
    }

    /// 创建张量类型
    pub fn tensor(element: NpuType, shape: Vec<i64>, layout: TensorLayout) -> Self {
        Self {
            kind: NpuTypeKind::Tensor {
                element: Box::new(element.kind),
                shape,
                layout,
            },
            device_hint: None,
        }
    }

    /// 创建 NCHW 张量
    pub fn tensor_nchw(element: NpuType, n: i64, c: i64, h: i64, w: i64) -> Self {
        Self::tensor(element, vec![n, c, h, w], TensorLayout::NCHW)
    }

    /// 创建 NHWC 张量
    pub fn tensor_nhwc(element: NpuType, n: i64, c: i64, h: i64, w: i64) -> Self {
        Self::tensor(element, vec![n, h, w, c], TensorLayout::NHWC)
    }

    /// 创建 void 类型
    pub fn void() -> Self {
        Self {
            kind: NpuTypeKind::Void,
            device_hint: None,
        }
    }

    /// 创建 bool 类型 (i1)
    pub fn bool() -> Self {
        Self::integer(1, false)
    }

    /// 获取类型大小（字节）
    pub fn size_in_bytes(&self) -> usize {
        match &self.kind {
            NpuTypeKind::Integer { width, .. } => (*width as usize + 7) / 8,
            NpuTypeKind::Float { width } => (*width as usize + 7) / 8,
            NpuTypeKind::Quantized { base, .. } => match base {
                QuantBase::Int8 | QuantBase::UInt8 | QuantBase::FP8 => 1,
                QuantBase::Int4 | QuantBase::UInt4 => 1, // 通常打包存储
                QuantBase::BF16 => 2,
            },
            NpuTypeKind::Tensor { element, shape, .. } => {
                let elem_size = Self { kind: (**element).clone(), device_hint: None }.size_in_bytes();
                let num_elements: i64 = shape.iter().filter(|d| **d > 0).product();
                elem_size * num_elements as usize
            }
            NpuTypeKind::TensorSlice { .. } => 0, // 切片不占用额外空间
            NpuTypeKind::Void => 0,
        }
    }

    /// 是否是量化类型
    pub fn is_quantized(&self) -> bool {
        matches!(&self.kind, NpuTypeKind::Quantized { .. })
    }

    /// 获取张量布局
    pub fn layout(&self) -> Option<TensorLayout> {
        match &self.kind {
            NpuTypeKind::Tensor { layout, .. } => Some(*layout),
            _ => None,
        }
    }

    /// 获取张量形状
    pub fn shape(&self) -> Option<&Vec<i64>> {
        match &self.kind {
            NpuTypeKind::Tensor { shape, .. } => Some(shape),
            _ => None,
        }
    }

    /// 获取元素类型
    pub fn element_type(&self) -> Option<NpuTypeKind> {
        match &self.kind {
            NpuTypeKind::Tensor { element, .. } => Some((**element).clone()),
            NpuTypeKind::TensorSlice { source, .. } => Some((**source).clone()),
            _ => None,
        }
    }

    /// 转换为 ONNX 类型字符串
    pub fn to_onnx_type(&self) -> String {
        match &self.kind {
            NpuTypeKind::Integer { width, signed } => {
                if *signed {
                    format!("int{}", width)
                } else {
                    format!("uint{}", width)
                }
            }
            NpuTypeKind::Float { width } => match width {
                16 => "float16".to_string(),
                32 => "float".to_string(),
                64 => "double".to_string(),
                _ => "float".to_string(),
            },
            NpuTypeKind::Quantized { base, scale, zero_point } => {
                match base {
                    QuantBase::Int8 => format!("tensor(int8), scale={}, zero_point={}", scale, zero_point),
                    QuantBase::UInt8 => format!("tensor(uint8), scale={}, zero_point={}", scale, zero_point),
                    QuantBase::Int4 => format!("tensor(int4), scale={}, zero_point={}", scale, zero_point),
                    QuantBase::UInt4 => format!("tensor(uint4), scale={}, zero_point={}", scale, zero_point),
                    QuantBase::FP8 => "float8".to_string(),
                    QuantBase::BF16 => "bfloat16".to_string(),
                }
            }
            NpuTypeKind::Tensor { element, .. } => {
                let elem = Self { kind: (**element).clone(), device_hint: None };
                elem.to_onnx_type()
            }
            NpuTypeKind::TensorSlice { .. } => "tensor".to_string(),
            NpuTypeKind::Void => "void".to_string(),
        }
    }

    /// 转换为 Python/NumPy 类型字符串
    pub fn to_numpy_dtype(&self) -> String {
        match &self.kind {
            NpuTypeKind::Integer { width, signed } => {
                let prefix = if *signed { "int" } else { "uint" };
                format!("{}{}", prefix, width)
            }
            NpuTypeKind::Float { width } => match width {
                16 => "float16".to_string(),
                32 => "float32".to_string(),
                64 => "float64".to_string(),
                _ => "float32".to_string(),
            },
            NpuTypeKind::Quantized { base, .. } => match base {
                QuantBase::Int8 => "int8".to_string(),
                QuantBase::UInt8 => "uint8".to_string(),
                QuantBase::BF16 => "bfloat16".to_string(),
                _ => "float32".to_string(),
            },
            _ => "float32".to_string(),
        }
    }

    /// 转换为 C 类型字符串
    pub fn to_c_type(&self) -> String {
        match &self.kind {
            NpuTypeKind::Integer { width, signed } => {
                if *signed {
                    match width {
                        8 => "int8_t".to_string(),
                        16 => "int16_t".to_string(),
                        32 => "int32_t".to_string(),
                        64 => "int64_t".to_string(),
                        _ => "int32_t".to_string(),
                    }
                } else {
                    match width {
                        8 => "uint8_t".to_string(),
                        16 => "uint16_t".to_string(),
                        32 => "uint32_t".to_string(),
                        64 => "uint64_t".to_string(),
                        _ => "uint32_t".to_string(),
                    }
                }
            }
            NpuTypeKind::Float { width } => match width {
                16 => "_Float16".to_string(),
                32 => "float".to_string(),
                64 => "double".to_string(),
                _ => "float".to_string(),
            },
            NpuTypeKind::Quantized { base, .. } => match base {
                QuantBase::Int8 => "int8_t".to_string(),
                QuantBase::UInt8 => "uint8_t".to_string(),
                QuantBase::BF16 => "bfloat16_t".to_string(),
                _ => "float".to_string(),
            },
            _ => "void".to_string(),
        }
    }
}

impl fmt::Display for NpuType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_onnx_type())
    }
}

/// 从 AST 类型创建 NPU 类型
impl From<&crate::ast::Type> for NpuType {
    fn from(ast_type: &crate::ast::Type) -> Self {
        match ast_type {
            crate::ast::Type::I8 => NpuType::i8(),
            crate::ast::Type::I16 => NpuType::i16(),
            crate::ast::Type::I32 => NpuType::i32(),
            crate::ast::Type::I64 => NpuType::i64(),
            crate::ast::Type::I128 => NpuType::integer(128, true),
            crate::ast::Type::U8 => NpuType::u8(),
            crate::ast::Type::U16 => NpuType::integer(16, false),
            crate::ast::Type::U32 => NpuType::u32(),
            crate::ast::Type::U64 => NpuType::integer(64, false),
            crate::ast::Type::U128 => NpuType::integer(128, false),
            crate::ast::Type::F32 => NpuType::f32(),
            crate::ast::Type::F64 => NpuType::f64(),
            crate::ast::Type::Bool => NpuType::bool(),
            crate::ast::Type::Char => NpuType::u8(),
            crate::ast::Type::Buffer(elem_type, dim) => {
                let elem = NpuType::from(elem_type.as_ref());
                let shape = dim.map(|d| vec![d as i64]).unwrap_or_default();
                NpuType::tensor(elem, shape, TensorLayout::NCHW)
            }
            crate::ast::Type::Named(name) => {
                match name.as_str() {
                    "f16" | "half" | "fp16" => NpuType::fp16(),
                    "bf16" | "bfloat16" => NpuType::bf16(),
                    "fp8" => NpuType::quant_fp8(1.0, 0),
                    "qint8" | "int8_q" => NpuType::quant_int8(1.0, 0),
                    _ => NpuType::f32(),
                }
            }
            crate::ast::Type::Tuple(_) => NpuType::void(),
        }
    }
}

/// NPU 支持的数据类型检查
impl NpuType {
    /// 检查 Intel NPU 是否支持该类型
    pub fn is_supported_by_intel_npu(&self) -> bool {
        match &self.kind {
            NpuTypeKind::Integer { width, .. } => *width <= 32,
            NpuTypeKind::Float { width } => *width == 16 || *width == 32,
            NpuTypeKind::Quantized { base, .. } => {
                matches!(base, QuantBase::Int8 | QuantBase::UInt8 | QuantBase::BF16)
            }
            NpuTypeKind::Tensor { element, .. } => {
                let elem = Self { kind: (**element).clone(), device_hint: None };
                elem.is_supported_by_intel_npu()
            }
            _ => false,
        }
    }

    /// 检查昇腾 NPU 是否支持该类型
    pub fn is_supported_by_ascend(&self) -> bool {
        match &self.kind {
            NpuTypeKind::Integer { width, .. } => *width <= 64,
            NpuTypeKind::Float { width } => *width == 16 || *width == 32,
            NpuTypeKind::Quantized { base, .. } => {
                matches!(base, QuantBase::Int8 | QuantBase::UInt8 | QuantBase::BF16 | QuantBase::Int4)
            }
            NpuTypeKind::Tensor { element, .. } => {
                let elem = Self { kind: (**element).clone(), device_hint: None };
                elem.is_supported_by_ascend()
            }
            _ => false,
        }
    }

    /// 检查 TPU 是否支持该类型
    pub fn is_supported_by_tpu(&self) -> bool {
        match &self.kind {
            NpuTypeKind::Integer { width, .. } => *width <= 64,
            NpuTypeKind::Float { width } => *width == 16 || *width == 32 || *width == 64,
            NpuTypeKind::Quantized { base, .. } => {
                matches!(base, QuantBase::Int8 | QuantBase::UInt8 | QuantBase::BF16)
            }
            NpuTypeKind::Tensor { element, .. } => {
                let elem = Self { kind: (**element).clone(), device_hint: None };
                elem.is_supported_by_tpu()
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_types() {
        let i32_type = NpuType::i32();
        assert!(matches!(i32_type.kind, NpuTypeKind::Integer { width: 32, signed: true }));
        assert_eq!(i32_type.to_onnx_type(), "int32");
        assert_eq!(i32_type.to_c_type(), "int32_t");
    }

    #[test]
    fn test_float_types() {
        let f32_type = NpuType::f32();
        assert!(matches!(f32_type.kind, NpuTypeKind::Float { width: 32 }));
        assert_eq!(f32_type.to_onnx_type(), "float");
        assert_eq!(f32_type.to_numpy_dtype(), "float32");
    }

    #[test]
    fn test_quantized_types() {
        let q8_type = NpuType::quant_int8(0.0078, 128);
        assert!(q8_type.is_quantized());
        assert!(q8_type.is_supported_by_intel_npu());
    }

    #[test]
    fn test_tensor_types() {
        let tensor = NpuType::tensor_nchw(NpuType::f32(), 1, 3, 224, 224);
        assert!(matches!(tensor.layout(), Some(TensorLayout::NCHW)));
        assert_eq!(tensor.shape(), Some(&vec![1, 3, 224, 224]));
    }

    #[test]
    fn test_device_support() {
        let fp64 = NpuType::f64();
        assert!(!fp64.is_supported_by_intel_npu()); // Intel NPU 不支持 FP64

        let fp32 = NpuType::f32();
        assert!(fp32.is_supported_by_intel_npu());
        assert!(fp32.is_supported_by_ascend());
        assert!(fp32.is_supported_by_tpu());
    }
}
