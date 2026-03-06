//! Triton 类型系统
//!
//! 定义 Triton 类型及其与 HSCIR 类型的映射关系。

use std::fmt;

/// Triton 类型种类
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TritonTypeKind {
    /// 整数类型 (tl.intN, tl.uintN)
    Integer,
    /// 浮点类型 (tl.float16, tl.float32, tl.float64)
    Float,
    /// 指针类型 (指针用于内存访问)
    Pointer,
    /// 张量/块类型 (tl.tensor)
    Tensor,
    /// void 类型
    Void,
}

/// Triton 类型
#[derive(Debug, Clone)]
pub struct TritonType {
    /// 类型种类
    pub kind: TritonTypeKind,
    /// 位宽 (用于整数和浮点类型)
    pub width: Option<u32>,
    /// 是否有符号 (用于整数类型)
    pub is_signed: bool,
    /// 元素类型 (用于指针和张量)
    pub element_type: Option<Box<TritonType>>,
    /// 形状 (用于张量类型)
    pub shape: Vec<i64>,
}

impl TritonType {
    /// 创建整数类型
    pub fn integer(width: u32, is_signed: bool) -> Self {
        Self {
            kind: TritonTypeKind::Integer,
            width: Some(width),
            is_signed,
            element_type: None,
            shape: vec![],
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

    /// 创建 u64 类型
    pub fn u64() -> Self {
        Self::integer(64, false)
    }

    /// 创建浮点类型
    pub fn float(width: u32) -> Self {
        Self {
            kind: TritonTypeKind::Float,
            width: Some(width),
            is_signed: true,
            element_type: None,
            shape: vec![],
        }
    }

    /// 创建 float16 类型
    pub fn float16() -> Self {
        Self::float(16)
    }

    /// 创建 float32 类型
    pub fn float32() -> Self {
        Self::float(32)
    }

    /// 创建 float64 类型
    pub fn float64() -> Self {
        Self::float(64)
    }

    /// 创建指针类型
    pub fn pointer(element_type: TritonType) -> Self {
        Self {
            kind: TritonTypeKind::Pointer,
            width: Some(64), // 64-bit pointer
            is_signed: false,
            element_type: Some(Box::new(element_type)),
            shape: vec![],
        }
    }

    /// 创建张量/块类型
    pub fn tensor(element_type: TritonType, shape: Vec<i64>) -> Self {
        Self {
            kind: TritonTypeKind::Tensor,
            width: None,
            is_signed: false,
            element_type: Some(Box::new(element_type)),
            shape,
        }
    }

    /// 创建 void 类型
    pub fn void() -> Self {
        Self {
            kind: TritonTypeKind::Void,
            width: None,
            is_signed: false,
            element_type: None,
            shape: vec![],
        }
    }

    /// 创建 bool 类型 (i1)
    pub fn bool() -> Self {
        Self::integer(1, false)
    }

    /// 获取 Triton Python 类型字符串
    pub fn to_triton_string(&self) -> String {
        match self.kind {
            TritonTypeKind::Integer => {
                let prefix = if self.is_signed { "int" } else { "uint" };
                format!("tl.{}{}", prefix, self.width.unwrap_or(32))
            }
            TritonTypeKind::Float => {
                match self.width {
                    Some(16) => "tl.float16".to_string(),
                    Some(32) => "tl.float32".to_string(),
                    Some(64) => "tl.float64".to_string(),
                    _ => "tl.float32".to_string(),
                }
            }
            TritonTypeKind::Pointer => {
                if let Some(ref elem) = self.element_type {
                    format!("*{}", elem.to_triton_string())
                } else {
                    "tl.int64".to_string() // 通用指针
                }
            }
            TritonTypeKind::Tensor => {
                if let Some(ref elem) = self.element_type {
                    let shape_str: Vec<String> = self.shape.iter()
                        .map(|d| {
                            if *d > 0 {
                                d.to_string()
                            } else {
                                "BLOCK_SIZE".to_string() // 动态维度
                            }
                        })
                        .collect();
                    if shape_str.is_empty() {
                        elem.to_triton_string()
                    } else {
                        // Triton 的块通常不需要显式的 shape 类型
                        elem.to_triton_string()
                    }
                } else {
                    "tl.float32".to_string()
                }
            }
            TritonTypeKind::Void => "None".to_string(),
        }
    }

    /// 获取 C 类型字符串 (用于宿主代码)
    pub fn to_c_type(&self) -> String {
        match self.kind {
            TritonTypeKind::Integer => {
                let prefix = if self.is_signed { "" } else { "unsigned " };
                match self.width {
                    Some(8) => format!("{}char", prefix),
                    Some(16) => format!("{}short", prefix),
                    Some(32) => format!("{}int", prefix),
                    Some(64) => format!("{}long long", prefix),
                    _ => "int".to_string(),
                }
            }
            TritonTypeKind::Float => {
                match self.width {
                    Some(16) => "__half".to_string(),
                    Some(32) => "float".to_string(),
                    Some(64) => "double".to_string(),
                    _ => "float".to_string(),
                }
            }
            TritonTypeKind::Pointer => {
                if let Some(ref elem) = self.element_type {
                    format!("{}*", elem.to_c_type())
                } else {
                    "void*".to_string()
                }
            }
            TritonTypeKind::Tensor => {
                if let Some(ref elem) = self.element_type {
                    elem.to_c_type()
                } else {
                    "float".to_string()
                }
            }
            TritonTypeKind::Void => "void".to_string(),
        }
    }

    /// 获取 Python 类型字符串 (用于类型注解)
    pub fn to_python_type(&self) -> String {
        match self.kind {
            TritonTypeKind::Integer => {
                match self.width {
                    Some(8) => "int8",
                    Some(16) => "int16",
                    Some(32) => "int32",
                    Some(64) => "int64",
                    _ => "int32",
                }.to_string()
            }
            TritonTypeKind::Float => {
                match self.width {
                    Some(16) => "float16",
                    Some(32) => "float32",
                    Some(64) => "float64",
                    _ => "float32",
                }.to_string()
            }
            TritonTypeKind::Pointer => "int64".to_string(), // 指针作为整数传递
            TritonTypeKind::Tensor => "torch.Tensor".to_string(),
            TritonTypeKind::Void => "None".to_string(),
        }
    }
}

impl fmt::Display for TritonType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_triton_string())
    }
}

/// 从 AST 类型创建 Triton 类型
impl From<&crate::ast::Type> for TritonType {
    fn from(ast_type: &crate::ast::Type) -> Self {
        match ast_type {
            crate::ast::Type::I8 => TritonType::i8(),
            crate::ast::Type::I16 => TritonType::i16(),
            crate::ast::Type::I32 => TritonType::i32(),
            crate::ast::Type::I64 => TritonType::i64(),
            crate::ast::Type::I128 => TritonType::integer(128, true),
            crate::ast::Type::U8 => TritonType::u8(),
            crate::ast::Type::U16 => TritonType::integer(16, false),
            crate::ast::Type::U32 => TritonType::u32(),
            crate::ast::Type::U64 => TritonType::u64(),
            crate::ast::Type::U128 => TritonType::integer(128, false),
            crate::ast::Type::F32 => TritonType::float32(),
            crate::ast::Type::F64 => TritonType::float64(),
            crate::ast::Type::Bool => TritonType::bool(),
            crate::ast::Type::Char => TritonType::u8(),
            crate::ast::Type::Buffer(elem_type, dim) => {
                let elem = TritonType::from(elem_type.as_ref());
                let shape = dim.map(|d| vec![d as i64]).unwrap_or_default();
                TritonType::tensor(elem, shape)
            }
            crate::ast::Type::Named(name) => {
                // 对于命名类型，默认使用 float32
                match name.as_str() {
                    "f16" | "half" => TritonType::float16(),
                    "bf16" | "bfloat16" => TritonType::float16(), // 简化处理
                    _ => TritonType::float32(),
                }
            }
            crate::ast::Type::Tuple(_) => TritonType::void(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_types() {
        let i32_type = TritonType::i32();
        assert_eq!(i32_type.to_triton_string(), "tl.int32");
        assert_eq!(i32_type.to_c_type(), "int");

        let u32_type = TritonType::u32();
        assert_eq!(u32_type.to_triton_string(), "tl.uint32");
        assert_eq!(u32_type.to_c_type(), "unsigned int");
    }

    #[test]
    fn test_float_types() {
        let f32_type = TritonType::float32();
        assert_eq!(f32_type.to_triton_string(), "tl.float32");
        assert_eq!(f32_type.to_c_type(), "float");

        let f16_type = TritonType::float16();
        assert_eq!(f16_type.to_triton_string(), "tl.float16");
        assert_eq!(f16_type.to_c_type(), "__half");
    }

    #[test]
    fn test_pointer_type() {
        let ptr_type = TritonType::pointer(TritonType::float32());
        assert_eq!(ptr_type.to_triton_string(), "*tl.float32");
        assert_eq!(ptr_type.to_c_type(), "float*");
    }

    #[test]
    fn test_tensor_type() {
        let tensor_type = TritonType::tensor(TritonType::float32(), vec![1024, 1024]);
        assert_eq!(tensor_type.to_triton_string(), "tl.float32");
    }

    #[test]
    fn test_ast_type_conversion() {
        use crate::ast::Type;

        let ast_i32 = Type::I32;
        let triton_i32 = TritonType::from(&ast_i32);
        assert_eq!(triton_i32.to_triton_string(), "tl.int32");

        let ast_f32 = Type::F32;
        let triton_f32 = TritonType::from(&ast_f32);
        assert_eq!(triton_f32.to_triton_string(), "tl.float32");

        let ast_buffer = Type::Buffer(Box::new(Type::F32), Some(1024));
        let triton_buffer = TritonType::from(&ast_buffer);
        assert_eq!(triton_buffer.kind, TritonTypeKind::Tensor);
    }
}
