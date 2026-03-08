//! CPU 后端类型系统
//!
//! 定义 HSCIR 类型到 C++ 类型的映射

use crate::ast::Type;

/// CPU 类型种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuTypeKind {
    /// 整数类型
    Integer,
    /// 浮点类型
    Float,
    /// 缓冲区类型
    Buffer,
    /// 函数类型
    Function,
    /// 空类型
    Void,
}

/// CPU 类型表示
#[derive(Debug, Clone)]
pub struct CpuType {
    /// 类型种类
    pub kind: CpuTypeKind,
    /// 类型名称（C++ 表示）
    pub name: String,
    /// 位宽（用于整数和浮点）
    pub width: Option<u32>,
    /// 是否有符号（用于整数）
    pub signed: Option<bool>,
    /// 元素类型（用于 Buffer）
    pub element_type: Option<Box<CpuType>>,
    /// 形状（用于 Buffer）
    pub shape: Vec<i64>,
}

impl CpuType {
    /// 创建整数类型
    pub fn integer(width: u32, signed: bool) -> Self {
        let name = match (width, signed) {
            (1, _) => "bool".to_string(),
            (8, true) => "int8_t".to_string(),
            (8, false) => "uint8_t".to_string(),
            (16, true) => "int16_t".to_string(),
            (16, false) => "uint16_t".to_string(),
            (32, true) => "int32_t".to_string(),
            (32, false) => "uint32_t".to_string(),
            (64, true) => "int64_t".to_string(),
            (64, false) => "uint64_t".to_string(),
            (128, true) => "__int128".to_string(),
            (128, false) => "unsigned __int128".to_string(),
            _ => format!("int{}_t", width),
        };

        CpuType {
            kind: CpuTypeKind::Integer,
            name,
            width: Some(width),
            signed: Some(signed),
            element_type: None,
            shape: vec![],
        }
    }

    /// 创建浮点类型
    pub fn float(width: u32) -> Self {
        let name = match width {
            16 => "float16_t".to_string(),  // 需要 <experimental/type_traits>
            32 => "float".to_string(),
            64 => "double".to_string(),
            _ => format!("float{}_t", width),
        };

        CpuType {
            kind: CpuTypeKind::Float,
            name,
            width: Some(width),
            signed: None,
            element_type: None,
            shape: vec![],
        }
    }

    /// 创建缓冲区类型
    pub fn buffer(element_type: CpuType, shape: Vec<i64>) -> Self {
        CpuType {
            kind: CpuTypeKind::Buffer,
            name: format!("Buffer<{}>", element_type.name),
            width: None,
            signed: None,
            element_type: Some(Box::new(element_type)),
            shape,
        }
    }

    /// 创建 void 类型
    pub fn void() -> Self {
        CpuType {
            kind: CpuTypeKind::Void,
            name: "void".to_string(),
            width: None,
            signed: None,
            element_type: None,
            shape: vec![],
        }
    }

    /// 从 AST 类型转换
    pub fn from_ast(ty: &Type) -> Self {
        match ty {
            Type::I8 => CpuType::integer(8, true),
            Type::I16 => CpuType::integer(16, true),
            Type::I32 => CpuType::integer(32, true),
            Type::I64 => CpuType::integer(64, true),
            Type::I128 => CpuType::integer(128, true),
            Type::U8 => CpuType::integer(8, false),
            Type::U16 => CpuType::integer(16, false),
            Type::U32 => CpuType::integer(32, false),
            Type::U64 => CpuType::integer(64, false),
            Type::U128 => CpuType::integer(128, false),
            Type::F32 => CpuType::float(32),
            Type::F64 => CpuType::float(64),
            Type::Bool => CpuType::integer(1, false),
            Type::Char => CpuType::integer(8, true),
            Type::Buffer(elem, dims) => {
                let elem_type = CpuType::from_ast(elem);
                let shape: Vec<i64> = dims.iter().map(|&d| d as i64).collect();
                CpuType::buffer(elem_type, shape)
            }
            Type::Named(name) => CpuType {
                kind: CpuTypeKind::Void,
                name: name.clone(),
                width: None,
                signed: None,
                element_type: None,
                shape: vec![],
            },
            Type::Tuple(_) => CpuType::void(),
        }
    }

    /// 获取 C++ 类型声明
    pub fn to_cpp(&self) -> String {
        self.name.clone()
    }

    /// 获取 C++ 指针类型
    pub fn to_cpp_ptr(&self) -> String {
        format!("{}*", self.name)
    }

    /// 获取 C++ 引用类型
    pub fn to_cpp_ref(&self) -> String {
        format!("{}&", self.name)
    }

    /// 获取 C++ const 引用类型
    pub fn to_cpp_const_ref(&self) -> String {
        format!("const {}&", self.name)
    }

    /// 检查是否是浮点类型
    pub fn is_float(&self) -> bool {
        self.kind == CpuTypeKind::Float
    }

    /// 检查是否是整数类型
    pub fn is_integer(&self) -> bool {
        self.kind == CpuTypeKind::Integer
    }

    /// 检查是否是缓冲区类型
    pub fn is_buffer(&self) -> bool {
        self.kind == CpuTypeKind::Buffer
    }

    /// 获取元素大小（字节）
    pub fn element_size(&self) -> usize {
        match self.kind {
            CpuTypeKind::Integer => self.width.unwrap_or(32) as usize / 8,
            CpuTypeKind::Float => self.width.unwrap_or(32) as usize / 8,
            CpuTypeKind::Buffer => {
                if let Some(ref elem) = self.element_type {
                    elem.element_size()
                } else {
                    0
                }
            }
            _ => 0,
        }
    }

    /// 计算缓冲区总大小（字节）
    pub fn buffer_size(&self) -> usize {
        if self.kind != CpuTypeKind::Buffer {
            return 0;
        }

        let elem_size = self.element_size();
        let total_elements: i64 = self.shape.iter().filter(|&&d| d > 0).product();
        elem_size * total_elements as usize
    }
}

/// 类型映射表
pub struct TypeMapper;

impl TypeMapper {
    /// 将 AST 类型映射到 CPU 类型
    pub fn map_type(ty: &Type) -> CpuType {
        CpuType::from_ast(ty)
    }

    /// 获取类型的 C++ 字面量后缀
    pub fn literal_suffix(ty: &CpuType) -> &'static str {
        match (ty.kind, ty.width) {
            (CpuTypeKind::Integer, Some(32)) => "",
            (CpuTypeKind::Integer, Some(64)) => "LL",
            (CpuTypeKind::Integer, Some(128)) => "LL",
            (CpuTypeKind::Float, Some(32)) => "f",
            (CpuTypeKind::Float, Some(64)) => "",
            _ => "",
        }
    }

    /// 获取类型的默认值
    pub fn default_value(ty: &CpuType) -> String {
        match ty.kind {
            CpuTypeKind::Integer => "0".to_string(),
            CpuTypeKind::Float => "0.0".to_string(),
            CpuTypeKind::Buffer => "Buffer<>()".to_string(),
            CpuTypeKind::Void => "".to_string(),
            _ => "".to_string(),
        }
    }

    /// 获取类型的格式化字符串（用于 printf）
    pub fn format_specifier(ty: &CpuType) -> &'static str {
        match (ty.kind, ty.width, ty.signed) {
            (CpuTypeKind::Integer, Some(1), _) => "%d",
            (CpuTypeKind::Integer, Some(8), Some(true)) => "%d",
            (CpuTypeKind::Integer, Some(8), Some(false)) => "%u",
            (CpuTypeKind::Integer, Some(16), Some(true)) => "%d",
            (CpuTypeKind::Integer, Some(16), Some(false)) => "%u",
            (CpuTypeKind::Integer, Some(32), Some(true)) => "%d",
            (CpuTypeKind::Integer, Some(32), Some(false)) => "%u",
            (CpuTypeKind::Integer, Some(64), Some(true)) => "%lld",
            (CpuTypeKind::Integer, Some(64), Some(false)) => "%llu",
            (CpuTypeKind::Float, Some(32), _) => "%f",
            (CpuTypeKind::Float, Some(64), _) => "%lf",
            _ => "%p",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_type() {
        let i32_type = CpuType::integer(32, true);
        assert_eq!(i32_type.name, "int32_t");
        assert!(i32_type.is_integer());

        let u64_type = CpuType::integer(64, false);
        assert_eq!(u64_type.name, "uint64_t");
    }

    #[test]
    fn test_float_type() {
        let f32_type = CpuType::float(32);
        assert_eq!(f32_type.name, "float");
        assert!(f32_type.is_float());

        let f64_type = CpuType::float(64);
        assert_eq!(f64_type.name, "double");
    }

    #[test]
    fn test_buffer_type() {
        let elem_type = CpuType::float(32);
        let buf_type = CpuType::buffer(elem_type, vec![100, 200]);
        assert!(buf_type.is_buffer());
        assert_eq!(buf_type.shape, vec![100, 200]);
    }

    #[test]
    fn test_type_from_ast() {
        let ast_i32 = Type::I32;
        let cpu_i32 = CpuType::from_ast(&ast_i32);
        assert_eq!(cpu_i32.name, "int32_t");

        let ast_f64 = Type::F64;
        let cpu_f64 = CpuType::from_ast(&ast_f64);
        assert_eq!(cpu_f64.name, "double");
    }
}
